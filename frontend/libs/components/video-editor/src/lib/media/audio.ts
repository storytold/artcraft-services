// Audio runtime. Three jobs: collect timeline candidates that should
// produce sound (collectAudibleCandidates / collectAudioMixSources /
// collectAudioClips), decode each source to an AudioBuffer in the
// playback context's sample rate, and mix everything into a single
// stereo master buffer for export (createTimelineAudioBuffer).
//
// Audio source resolution covers three cases:
//   1. Uploaded audio file       — decode the asset's File directly
//   2. Library (URL) audio       — fetch + decode the network blob
//   3. Video clip's source audio — pull the audio track out via
//      mediabunny's AudioBufferSink, resample to playback context's
//      rate via OfflineAudioContext.
//
// Per-channel mixing in mixAudioChannels does its own linear
// interpolation between source samples to honor retime rate
// without an extra rendering pass (when pitch-preservation is off).
// When pitch must be preserved, renderRetimedBuffer runs a SoundTouch
// pre-pass and mixAudioChannels reads the pre-rendered buffer at 1:1.
//
// The export bus runs through applyAudioMasteringToBuffer at the end,
// so overlapping loud clips don't clip.

import { Input, ALL_FORMATS, BlobSource, AudioBufferSink } from "mediabunny";

import type { MediaAsset } from "./types";
import { applyAudioMasteringToBuffer } from "./audio-mastering";
import { mediaSupportsAudio } from "./media-utils";
import { computeRmsBuckets, type SampleBucket } from "./waveform-summary";
import { clampRetimeRate, shouldMaintainPitch } from "../retime/rate";
import { getSourceTimeAtClipTime } from "../retime/resolve";
import { renderRetimedBuffer } from "../retime/audio-stretch";
import type {
  AudioElement,
  VideoElement,
  LibraryAudioElement,
  RetimeConfig,
  SceneTracks,
} from "../timeline/types";
import type { AudioCapableElement } from "../timeline/audio-state";
import {
  createAudioGainResolver,
  isElementMuted,
  resolveEffectiveAudioGain,
} from "../timeline/audio-state";
import { doesElementHaveEnabledAudio } from "../timeline/audio-separation";
import { canElementHaveAudio, hasMediaId } from "../timeline/element-utils";
import { canTrackHaveAudio } from "../timeline/track-capabilities";
import { TICKS_PER_SECOND } from "../wasm";

const MAX_AUDIO_CHANNELS = 2;
const EXPORT_SAMPLE_RATE = 44100;

// Longest timeline the export mixer will allocate a master buffer for (one
// hour; ~1.3 GB of stereo float samples at 44.1 kHz). The duration comes from
// element timing in the project document, so it is checked before any
// decoding or allocation.
export const MAX_TIMELINE_AUDIO_SECONDS = 60 * 60;

export interface CollectedAudioElement {
  timelineElement: AudioCapableElement;
  buffer: AudioBuffer;
  startTime: number;
  duration: number;
  trimStart: number;
  trimEnd: number;
  volume: number;
  muted: boolean;
  retime?: RetimeConfig;
}

export interface DecodedAudio {
  samples: Float32Array;
  sampleRate: number;
}

export interface AudibleElementCandidate {
  element: AudioElement | VideoElement;
  mediaAsset: MediaAsset | null;
}

export interface AudioClipSource {
  timelineElement: AudioCapableElement;
  id: string;
  sourceKey: string;
  file: File;
  startTime: number;
  duration: number;
  trimStart: number;
  trimEnd: number;
  volume: number;
  muted: boolean;
  retime?: RetimeConfig;
}

interface AudioMixSource {
  timelineElement: AudioCapableElement;
  file: File;
  startTime: number;
  duration: number;
  trimStart: number;
  trimEnd: number;
  volume: number;
  retime?: RetimeConfig;
}

export function createAudioContext({
  sampleRate,
}: {
  sampleRate?: number;
} = {}): AudioContext {
  const AudioContextConstructor =
    window.AudioContext ||
    (window as typeof window & { webkitAudioContext?: typeof AudioContext })
      .webkitAudioContext;

  return new AudioContextConstructor(sampleRate ? { sampleRate } : undefined);
}

export async function decodeAudioToFloat32({
  audioBlob,
  sampleRate,
}: {
  audioBlob: Blob;
  sampleRate?: number;
}): Promise<DecodedAudio> {
  const audioContext = createAudioContext({ sampleRate });
  const arrayBuffer = await audioBlob.arrayBuffer();
  const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);

  const numChannels = audioBuffer.numberOfChannels;
  const length = audioBuffer.length;
  const samples = new Float32Array(length);

  for (let i = 0; i < length; i++) {
    let sum = 0;
    for (let channel = 0; channel < numChannels; channel++) {
      sum += audioBuffer.getChannelData(channel)[i];
    }
    samples[i] = sum / numChannels;
  }

  return { samples, sampleRate: audioBuffer.sampleRate };
}

export function collectAudibleCandidates({
  tracks,
  mediaAssets,
}: {
  tracks: SceneTracks;
  mediaAssets: MediaAsset[];
}): AudibleElementCandidate[] {
  const allTracks = [...tracks.overlay, tracks.main, ...tracks.audio];
  const mediaMap = new Map(mediaAssets.map((a) => [a.id, a]));
  const candidates: AudibleElementCandidate[] = [];

  for (const track of allTracks) {
    if (canTrackHaveAudio(track) && track.muted) continue;

    for (const element of track.elements) {
      if (!canElementHaveAudio(element)) continue;
      if (element.duration <= 0) continue;

      const mediaAsset = hasMediaId(element)
        ? (mediaMap.get(element.mediaId) ?? null)
        : null;
      if (!doesElementHaveEnabledAudio({ element, mediaAsset })) continue;

      candidates.push({ element, mediaAsset });
    }
  }

  return candidates;
}

export function timelineHasAudio({
  tracks,
  mediaAssets,
}: {
  tracks: SceneTracks;
  mediaAssets: MediaAsset[];
}): boolean {
  return collectAudibleCandidates({ tracks, mediaAssets }).some(
    ({ element }) => !isElementMuted({ element }),
  );
}

export async function collectAudioElements({
  tracks,
  mediaAssets,
  audioContext,
}: {
  tracks: SceneTracks;
  mediaAssets: MediaAsset[];
  audioContext: AudioContext;
}): Promise<CollectedAudioElement[]> {
  const candidates = collectAudibleCandidates({ tracks, mediaAssets });
  const mediaMap = new Map<string, MediaAsset>(
    mediaAssets.map((media) => [media.id, media]),
  );
  const pendingElements: Array<Promise<CollectedAudioElement | null>> = [];
  // One decode per source: many clips can reference the same upload or
  // library URL, and decoding (and retaining) a full copy per clip multiplied
  // memory and work by the clip count.
  const decodedSources = new Map<string, Promise<AudioBuffer | null>>();
  const decodeOnce = (
    key: string,
    decode: () => Promise<AudioBuffer | null>,
  ): Promise<AudioBuffer | null> => {
    let pending = decodedSources.get(key);
    if (!pending) {
      pending = decode();
      decodedSources.set(key, pending);
    }
    return pending;
  };

  for (const { element, mediaAsset } of candidates) {
    if (element.type === "audio") {
      const sourceKey =
        element.sourceType === "upload"
          ? `asset:${element.mediaId}`
          : `url:${element.sourceUrl}`;
      const decode = () =>
        resolveAudioBufferForElement({
          element,
          mediaMap,
          audioContext,
        });
      pendingElements.push(
        (element.sourceType !== "upload" && element.buffer
          ? decode()
          : decodeOnce(sourceKey, decode)
        ).then((audioBuffer) => {
          if (!audioBuffer) return null;
          return {
            timelineElement: element,
            buffer: audioBuffer,
            startTime: element.startTime / TICKS_PER_SECOND,
            duration: element.duration / TICKS_PER_SECOND,
            trimStart: element.trimStart / TICKS_PER_SECOND,
            trimEnd: element.trimEnd / TICKS_PER_SECOND,
            volume: resolveEffectiveAudioGain({
              element,
              trackMuted: false,
              localTime: 0,
            }),
            muted: isElementMuted({ element }),
            retime: element.retime,
          };
        }),
      );
      continue;
    }

    if (element.type === "video") {
      if (!mediaAsset || !mediaSupportsAudio({ media: mediaAsset })) continue;

      pendingElements.push(
        decodeOnce(`asset:${mediaAsset.id}`, () =>
          resolveAudioBufferForAsset({
            asset: mediaAsset,
            audioContext,
          }),
        ).then((audioBuffer) => {
          if (!audioBuffer) return null;
          return {
            timelineElement: element,
            buffer: audioBuffer,
            startTime: element.startTime / TICKS_PER_SECOND,
            duration: element.duration / TICKS_PER_SECOND,
            trimStart: element.trimStart / TICKS_PER_SECOND,
            trimEnd: element.trimEnd / TICKS_PER_SECOND,
            volume: resolveEffectiveAudioGain({
              element,
              trackMuted: false,
              localTime: 0,
            }),
            muted: isElementMuted({ element }),
            retime: element.retime,
          };
        }),
      );
    }
  }

  const resolvedElements = await Promise.all(pendingElements);
  const audioElements: CollectedAudioElement[] = [];
  for (const element of resolvedElements) {
    if (element) audioElements.push(element);
  }
  return audioElements;
}

export async function collectAudioMixSources({
  tracks,
  mediaAssets,
}: {
  tracks: SceneTracks;
  mediaAssets: MediaAsset[];
}): Promise<AudioMixSource[]> {
  const orderedTracks = [...tracks.overlay, tracks.main, ...tracks.audio];
  const audioMixSources: AudioMixSource[] = [];
  const mediaMap = new Map<string, MediaAsset>(
    mediaAssets.map((asset) => [asset.id, asset]),
  );
  const pendingLibrarySources: Array<Promise<AudioMixSource | null>> = [];

  for (const track of orderedTracks) {
    if (canTrackHaveAudio(track) && track.muted) continue;

    for (const element of track.elements) {
      if (!canElementHaveAudio(element)) continue;
      if (isElementMuted({ element })) continue;
      const mediaAsset = hasMediaId(element)
        ? (mediaMap.get(element.mediaId) ?? null)
        : null;
      if (!doesElementHaveEnabledAudio({ element, mediaAsset })) continue;
      const volume = resolveEffectiveAudioGain({
        element,
        localTime: 0,
      });

      if (element.type === "audio") {
        if (element.sourceType === "upload") {
          const uploadAsset = mediaMap.get(element.mediaId);
          if (!uploadAsset) continue;

          audioMixSources.push(
            collectMediaAudioSource({ element, mediaAsset: uploadAsset, volume }),
          );
        } else {
          pendingLibrarySources.push(
            fetchLibraryAudioSource({ element, volume }),
          );
        }
        continue;
      }

      if (element.type === "video") {
        if (mediaAsset && mediaSupportsAudio({ media: mediaAsset })) {
          audioMixSources.push(
            collectMediaAudioSource({ element, mediaAsset, volume }),
          );
        }
      }
    }
  }

  const resolvedLibrarySources = await Promise.all(pendingLibrarySources);
  for (const source of resolvedLibrarySources) {
    if (source) audioMixSources.push(source);
  }

  return audioMixSources;
}

export async function collectAudioClips({
  tracks,
  mediaAssets,
}: {
  tracks: SceneTracks;
  mediaAssets: MediaAsset[];
}): Promise<AudioClipSource[]> {
  const orderedTracks = [...tracks.overlay, tracks.main, ...tracks.audio];
  const clips: AudioClipSource[] = [];
  const mediaMap = new Map<string, MediaAsset>(
    mediaAssets.map((asset) => [asset.id, asset]),
  );
  const pendingLibraryClips: Array<Promise<AudioClipSource | null>> = [];

  for (const track of orderedTracks) {
    const isTrackMuted = canTrackHaveAudio(track) && track.muted;

    for (const element of track.elements) {
      if (!canElementHaveAudio(element)) continue;

      const mediaAsset = hasMediaId(element)
        ? (mediaMap.get(element.mediaId) ?? null)
        : null;
      if (!doesElementHaveEnabledAudio({ element, mediaAsset })) continue;

      const muted = isTrackMuted || isElementMuted({ element });
      const volume = resolveEffectiveAudioGain({
        element,
        trackMuted: isTrackMuted,
        localTime: 0,
      });

      if (element.type === "audio") {
        if (element.sourceType === "upload") {
          const uploadAsset = mediaMap.get(element.mediaId);
          if (!uploadAsset) continue;

          clips.push(
            collectMediaAudioClip({
              element,
              mediaAsset: uploadAsset,
              muted,
              volume,
            }),
          );
        } else {
          pendingLibraryClips.push(
            fetchLibraryAudioClip({ element, muted, volume }),
          );
        }
        continue;
      }

      if (element.type === "video") {
        if (mediaAsset && mediaSupportsAudio({ media: mediaAsset })) {
          clips.push(
            collectMediaAudioClip({
              element,
              mediaAsset,
              muted,
              volume,
            }),
          );
        }
      }
    }
  }

  const resolvedLibraryClips = await Promise.all(pendingLibraryClips);
  for (const clip of resolvedLibraryClips) {
    if (clip) clips.push(clip);
  }

  return clips;
}

export async function createTimelineAudioBuffer({
  tracks,
  mediaAssets,
  duration,
  sampleRate = EXPORT_SAMPLE_RATE,
  audioContext,
}: {
  tracks: SceneTracks;
  mediaAssets: MediaAsset[];
  duration: number;
  sampleRate?: number;
  audioContext?: AudioContext;
}): Promise<AudioBuffer | null> {
  const durationSeconds = duration / TICKS_PER_SECOND;
  if (!Number.isFinite(durationSeconds) || durationSeconds < 0) {
    throw new Error("Invalid timeline duration for audio export");
  }
  if (durationSeconds > MAX_TIMELINE_AUDIO_SECONDS) {
    throw new Error(
      `Timeline is too long to export audio (limit ${MAX_TIMELINE_AUDIO_SECONDS / 60} minutes)`,
    );
  }

  const context = audioContext ?? createAudioContext({ sampleRate });

  const audioElements = await collectAudioElements({
    tracks,
    mediaAssets,
    audioContext: context,
  });

  if (audioElements.length === 0) return null;

  const outputChannels = 2;
  const outputLength = Math.max(1, Math.ceil(durationSeconds * sampleRate));
  const outputBuffer = context.createBuffer(
    outputChannels,
    outputLength,
    sampleRate,
  );

  for (const element of audioElements) {
    if (element.muted) continue;

    const pitchPreserved = shouldMaintainPitch({
      rate: element.retime?.rate ?? 1,
      maintainPitch: element.retime?.maintainPitch,
    });
    // Pitch-preserving preparation is expensive, so only the part of the clip
    // that lands inside the export is prepared (clip-local time `from`..`to`).
    const from = Math.max(0, -element.startTime);
    const to = Math.min(element.duration, durationSeconds - element.startTime);
    if (pitchPreserved && !(to > from)) continue;
    const renderedBuffer = pitchPreserved
      ? await renderRetimedBuffer({
          audioContext: context,
          sourceBuffer: element.buffer,
          trimStart: element.trimStart + from * clampRetimeRate({ rate: element.retime?.rate ?? 1 }),
          clipDuration: to - from,
          retime: element.retime,
          maintainPitch: true,
        })
      : undefined;

    mixAudioChannels({
      element,
      buffer: renderedBuffer ?? element.buffer,
      trimStart: renderedBuffer ? 0 : element.trimStart,
      retime: renderedBuffer ? undefined : element.retime,
      bufferOffset: renderedBuffer ? from : 0,
      outputBuffer,
      outputLength,
      sampleRate,
    });
  }

  return await applyAudioMasteringToBuffer({ audioBuffer: outputBuffer });
}

export function extractPeakRange({
  buffer,
  count,
  startSample,
  endSample,
}: {
  buffer: AudioBuffer;
  count: number;
  startSample: number;
  endSample: number;
}): number[] {
  return Array.from(
    collectPeakRange({
      buffer,
      count,
      startSample,
      endSample,
    }),
  );
}

export function getSampleBucketRange({
  startSample,
  endSample,
  bucketIndex,
  bucketCount,
}: {
  startSample: number;
  endSample: number;
  bucketIndex: number;
  bucketCount: number;
}): {
  bucketStart: number;
  bucketEnd: number;
} {
  const rangeLength = Math.max(0, endSample - startSample);
  const bucketStart =
    startSample + Math.floor((bucketIndex * rangeLength) / bucketCount);
  const bucketEnd =
    startSample + Math.floor(((bucketIndex + 1) * rangeLength) / bucketCount);
  return {
    bucketStart,
    bucketEnd: Math.max(bucketStart, bucketEnd),
  };
}

export function extractRmsBuckets({
  buffer,
  buckets,
}: {
  buffer: AudioBuffer;
  buckets: SampleBucket[];
}): number[] {
  return computeRmsBuckets({ buffer, buckets });
}

// Computes per-bucket waveform amplitude using the maximum RMS over a
// short analysis window inside each bucket. A naive mean-RMS over a
// whole bucket averages silence with nearby sound and smears
// transitions; taking the max over fixed short windows preserves smooth
// non-jittery RMS character while making transitions land where they
// actually happen.
export function extractRmsRange({
  buffer,
  count,
  startSample,
  endSample,
}: {
  buffer: AudioBuffer;
  count: number;
  startSample: number;
  endSample: number;
}): number[] {
  return extractRmsBuckets({
    buffer,
    buckets: Array.from({ length: count }, (_, bucketIndex) =>
      getSampleBucketRange({
        startSample,
        endSample,
        bucketIndex,
        bucketCount: count,
      }),
    ),
  });
}

async function resolveAudioBufferForElement({
  element,
  mediaMap,
  audioContext,
}: {
  element: AudioElement;
  mediaMap: Map<string, MediaAsset>;
  audioContext: AudioContext;
}): Promise<AudioBuffer | null> {
  try {
    if (element.sourceType === "upload") {
      const asset = mediaMap.get(element.mediaId);
      if (!asset) return null;
      return await resolveAudioBufferForAsset({ asset, audioContext });
    }

    if (element.buffer) return element.buffer;

    const response = await fetch(element.sourceUrl);
    if (!response.ok) {
      throw new Error(`Library audio fetch failed: ${response.status}`);
    }

    const arrayBuffer = await response.arrayBuffer();
    return await audioContext.decodeAudioData(arrayBuffer.slice(0));
  } catch (error) {
    console.warn("Failed to decode audio:", error);
    return null;
  }
}

async function resolveAudioBufferForAsset({
  asset,
  audioContext,
}: {
  asset: MediaAsset;
  audioContext: AudioContext;
}): Promise<AudioBuffer | null> {
  if (asset.type === "audio") {
    try {
      const arrayBuffer = await asset.file.arrayBuffer();
      return await audioContext.decodeAudioData(arrayBuffer.slice(0));
    } catch (error) {
      console.warn("Failed to decode audio asset:", error);
      return null;
    }
  }

  const input = new Input({
    source: new BlobSource(asset.file),
    formats: ALL_FORMATS,
  });

  try {
    const audioTrack = await input.getPrimaryAudioTrack();
    if (!audioTrack) return null;

    const sink = new AudioBufferSink(audioTrack);
    const targetSampleRate = audioContext.sampleRate;

    const chunks: AudioBuffer[] = [];
    let totalSamples = 0;

    for await (const { buffer } of sink.buffers(0)) {
      chunks.push(buffer);
      totalSamples += buffer.length;
    }

    if (chunks.length === 0) return null;

    const nativeSampleRate = chunks[0].sampleRate;
    const numChannels = Math.min(
      MAX_AUDIO_CHANNELS,
      chunks[0].numberOfChannels,
    );

    const nativeChannels = Array.from(
      { length: numChannels },
      () => new Float32Array(totalSamples),
    );
    let offset = 0;
    for (const chunk of chunks) {
      for (let channel = 0; channel < numChannels; channel++) {
        const sourceData = chunk.getChannelData(
          Math.min(channel, chunk.numberOfChannels - 1),
        );
        nativeChannels[channel].set(sourceData, offset);
      }
      offset += chunk.length;
    }

    // OfflineAudioContext gives high-quality resampling to playback rate.
    const outputSamples = Math.ceil(
      totalSamples * (targetSampleRate / nativeSampleRate),
    );
    const offlineContext = new OfflineAudioContext(
      numChannels,
      outputSamples,
      targetSampleRate,
    );

    const nativeBuffer = audioContext.createBuffer(
      numChannels,
      totalSamples,
      nativeSampleRate,
    );
    for (let ch = 0; ch < numChannels; ch++) {
      nativeBuffer.copyToChannel(nativeChannels[ch], ch);
    }

    const sourceNode = offlineContext.createBufferSource();
    sourceNode.buffer = nativeBuffer;
    sourceNode.connect(offlineContext.destination);
    sourceNode.start(0);

    return await offlineContext.startRendering();
  } catch (error) {
    console.warn("Failed to decode asset audio:", error);
    return null;
  } finally {
    input.dispose();
  }
}

async function fetchLibraryAudioSource({
  element,
  volume,
}: {
  element: LibraryAudioElement;
  volume: number;
}): Promise<AudioMixSource | null> {
  try {
    const response = await fetch(element.sourceUrl);
    if (!response.ok) {
      throw new Error(`Library audio fetch failed: ${response.status}`);
    }

    const blob = await response.blob();
    const file = new File([blob], `${element.name}.mp3`, {
      type: "audio/mpeg",
    });

    return {
      timelineElement: element,
      file,
      startTime: element.startTime / TICKS_PER_SECOND,
      duration: element.duration / TICKS_PER_SECOND,
      trimStart: element.trimStart / TICKS_PER_SECOND,
      trimEnd: element.trimEnd / TICKS_PER_SECOND,
      volume,
      retime: element.retime,
    };
  } catch (error) {
    console.warn("Failed to fetch library audio:", error);
    return null;
  }
}

async function fetchLibraryAudioClip({
  element,
  muted,
  volume,
}: {
  element: LibraryAudioElement;
  muted: boolean;
  volume: number;
}): Promise<AudioClipSource | null> {
  try {
    const response = await fetch(element.sourceUrl);
    if (!response.ok) {
      throw new Error(`Library audio fetch failed: ${response.status}`);
    }

    const blob = await response.blob();
    const file = new File([blob], `${element.name}.mp3`, {
      type: "audio/mpeg",
    });

    return {
      timelineElement: element,
      id: element.id,
      sourceKey: element.id,
      file,
      // Playback works in seconds, like collectMediaAudioClip. Passing raw
      // ticks here made prepared buffers TICKS_PER_SECOND times too long.
      startTime: element.startTime / TICKS_PER_SECOND,
      duration: element.duration / TICKS_PER_SECOND,
      trimStart: element.trimStart / TICKS_PER_SECOND,
      trimEnd: element.trimEnd / TICKS_PER_SECOND,
      volume,
      muted,
      retime: element.retime,
    };
  } catch (error) {
    console.warn("Failed to fetch library audio:", error);
    return null;
  }
}

function collectMediaAudioSource({
  element,
  mediaAsset,
  volume,
}: {
  element: AudioCapableElement;
  mediaAsset: MediaAsset;
  volume: number;
}): AudioMixSource {
  return {
    timelineElement: element,
    file: mediaAsset.file,
    startTime: element.startTime / TICKS_PER_SECOND,
    duration: element.duration / TICKS_PER_SECOND,
    trimStart: element.trimStart / TICKS_PER_SECOND,
    trimEnd: element.trimEnd / TICKS_PER_SECOND,
    volume,
    retime: element.retime,
  };
}

function collectMediaAudioClip({
  element,
  mediaAsset,
  muted,
  volume,
}: {
  element: AudioCapableElement;
  mediaAsset: MediaAsset;
  muted: boolean;
  volume: number;
}): AudioClipSource {
  return {
    timelineElement: element,
    id: element.id,
    sourceKey: mediaAsset.id,
    file: mediaAsset.file,
    startTime: element.startTime / TICKS_PER_SECOND,
    duration: element.duration / TICKS_PER_SECOND,
    trimStart: element.trimStart / TICKS_PER_SECOND,
    trimEnd: element.trimEnd / TICKS_PER_SECOND,
    volume,
    muted,
    retime: element.retime,
  };
}

function collectPeakRange({
  buffer,
  count,
  startSample,
  endSample,
}: {
  buffer: AudioBuffer;
  count: number;
  startSample: number;
  endSample: number;
}): Float32Array {
  const channels = buffer.numberOfChannels;
  const peaks = new Float32Array(count);

  for (let c = 0; c < channels; c++) {
    const data = buffer.getChannelData(c);
    for (let i = 0; i < count; i++) {
      const { bucketStart: start, bucketEnd: end } = getSampleBucketRange({
        startSample,
        endSample,
        bucketIndex: i,
        bucketCount: count,
      });
      for (let j = start; j < end; j++) {
        const abs = Math.abs(data[j]);
        if (abs > peaks[i]) peaks[i] = abs;
      }
    }
  }

  return peaks;
}

function mixAudioChannels({
  element,
  buffer,
  trimStart,
  retime,
  bufferOffset = 0,
  outputBuffer,
  outputLength,
  sampleRate,
}: {
  element: CollectedAudioElement;
  buffer: AudioBuffer;
  trimStart: number;
  retime?: RetimeConfig;
  /** Clip time at which `buffer` starts (a prepared buffer covering only part of the clip). */
  bufferOffset?: number;
  outputBuffer: AudioBuffer;
  outputLength: number;
  sampleRate: number;
}): void {
  const { startTime, duration: elementDuration } = element;
  if (
    !Number.isFinite(startTime) ||
    !Number.isFinite(elementDuration) ||
    elementDuration <= 0
  ) {
    return;
  }
  const safeTrimStart = Number.isFinite(trimStart) ? Math.max(0, trimStart) : 0;

  const outputStartSample = Math.floor(startTime * sampleRate);
  const renderedLength = Math.ceil(elementDuration * sampleRate);

  // Only walk the part of the clip that lands inside the output buffer. A
  // clip starting far before 0 (restored from a project document) used to
  // iterate every leading sample just to discard it.
  const firstSample = Math.max(0, -outputStartSample, Math.ceil(bufferOffset * sampleRate));
  const endSample = Math.min(renderedLength, outputLength - outputStartSample);
  if (firstSample >= endSample) return;

  const outputChannels = 2;
  const outputData = Array.from({ length: outputChannels }, (_, channel) =>
    outputBuffer.getChannelData(channel),
  );
  const sourceData = Array.from({ length: outputChannels }, (_, channel) =>
    buffer.getChannelData(Math.min(channel, buffer.numberOfChannels - 1)),
  );
  const sourceLength = sourceData[0].length;
  // Volume keyframes are normalized once, and the gain is evaluated once per
  // sample for both channels.
  const resolveGain = createAudioGainResolver({
    element: element.timelineElement,
  });

  for (let i = firstSample; i < endSample; i++) {
    const clipTime = i / sampleRate;
    const sourceTime =
      safeTrimStart + getSourceTimeAtClipTime({ clipTime: clipTime - bufferOffset, retime });
    const sourceIndex = sourceTime * buffer.sampleRate;
    if (sourceIndex >= sourceLength) break;

    const lowerIndex = Math.floor(sourceIndex);
    const upperIndex = Math.min(sourceLength - 1, lowerIndex + 1);
    const fraction = sourceIndex - lowerIndex;
    const gain = resolveGain(clipTime);
    const outputIndex = outputStartSample + i;
    for (let channel = 0; channel < outputChannels; channel++) {
      const source = sourceData[channel];
      outputData[channel][outputIndex] +=
        (source[lowerIndex] * (1 - fraction) + source[upperIndex] * fraction) *
        gain;
    }
  }
}
