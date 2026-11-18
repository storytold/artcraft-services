import { PitchShifter } from "soundtouchjs";
import { clampRetimeRate, shouldMaintainPitch } from "./rate";
import type { RetimeConfig } from "../timeline/types";
import {
  getSourceTimeAtClipTime,
  getTimelineDurationForSourceSpan,
} from "./resolve";

// Audio retime renderer. Two paths:
//   - Simple resampling (buildResampledBuffer): walk the output samples
//     and linear-interpolate the source samples at the corresponding
//     source time. Pitch shifts with the rate — fine for sound effects.
//   - Pitch-preserved (buildPitchPreservedBuffer): two-stage —
//     resample to native sample rate, then time-stretch with SoundTouch's
//     PitchShifter at rate `r` with pitch `1`. Output rate stays in
//     target sample rate; pitch stays unchanged.
//
// SoundTouch reads raw Float32 channel data and ignores the source
// buffer's native sample rate, hence the explicit resample-first step.

const RATE_EPSILON = 1e-6;

// Longest single clip rendered into a prepared buffer (one hour; ~1.3 GB of
// stereo float samples at 48 kHz). Clip timing comes from the project
// document, so without a cap a tiny project could request an arbitrarily
// large buffer and fill it synchronously.
export const MAX_RETIMED_CLIP_SECONDS = 60 * 60;

/**
 * Clip-time length worth rendering: the requested clip duration, cut off
 * where the source audio runs out (everything after that is silence) and at
 * MAX_RETIMED_CLIP_SECONDS. Non-finite or negative inputs yield 0.
 */
export function getRenderableClipDuration({
  sourceDuration,
  trimStart,
  clipDuration,
  retime,
}: {
  sourceDuration: number;
  trimStart: number;
  clipDuration: number;
  retime?: RetimeConfig;
}): number {
  if (!Number.isFinite(clipDuration) || clipDuration <= 0) {
    return 0;
  }

  const sourceRemaining = Math.max(0, sourceDuration - trimStart);
  const sourceLimited = getTimelineDurationForSourceSpan({
    sourceSpan: sourceRemaining,
    retime,
  });
  return Math.min(clipDuration, sourceLimited, MAX_RETIMED_CLIP_SECONDS);
}

function sanitizeTrimStart({ trimStart }: { trimStart: number }): number {
  return Number.isFinite(trimStart) ? Math.max(0, trimStart) : 0;
}

function sampleLinear({
  channelData,
  position,
}: {
  channelData: Float32Array;
  position: number;
}): number {
  if (position <= 0) {
    return channelData[0] ?? 0;
  }
  const lower = Math.floor(position);
  const upper = Math.min(channelData.length - 1, lower + 1);
  if (lower >= channelData.length) {
    return 0;
  }
  const fraction = position - lower;
  return channelData[lower] * (1 - fraction) + channelData[upper] * fraction;
}

function buildResampledBuffer({
  audioContext,
  sourceBuffer,
  trimStart,
  clipDuration,
  targetSampleRate,
  retime,
}: {
  audioContext: BaseAudioContext;
  sourceBuffer: AudioBuffer;
  trimStart: number;
  clipDuration: number;
  targetSampleRate: number;
  retime?: RetimeConfig;
}): AudioBuffer {
  const outputLength = Math.max(1, Math.ceil(clipDuration * targetSampleRate));
  const numChannels = Math.max(1, Math.min(2, sourceBuffer.numberOfChannels));
  const outputBuffer = audioContext.createBuffer(
    numChannels,
    outputLength,
    targetSampleRate,
  );

  for (let channel = 0; channel < numChannels; channel++) {
    const sourceData = sourceBuffer.getChannelData(
      Math.min(channel, sourceBuffer.numberOfChannels - 1),
    );
    const outputData = outputBuffer.getChannelData(channel);

    for (let i = 0; i < outputLength; i++) {
      const clipTime = i / targetSampleRate;
      const sourceTime =
        trimStart + getSourceTimeAtClipTime({ clipTime, retime });
      outputData[i] = sampleLinear({
        channelData: sourceData,
        position: sourceTime * sourceBuffer.sampleRate,
      });
    }
  }

  return outputBuffer;
}

async function buildPitchPreservedBuffer({
  sourceBuffer,
  trimStart,
  clipDuration,
  rate,
  targetSampleRate,
}: {
  sourceBuffer: AudioBuffer;
  trimStart: number;
  clipDuration: number;
  rate: number;
  targetSampleRate: number;
}): Promise<AudioBuffer> {
  const nativeSampleRate = sourceBuffer.sampleRate;
  const sourceDuration = clipDuration * rate;
  const startSample = Math.max(0, Math.floor(trimStart * nativeSampleRate));
  const numSourceSamples = Math.max(
    1,
    Math.ceil(sourceDuration * nativeSampleRate),
  );
  const available = Math.max(0, sourceBuffer.length - startSample);
  const actualSamples = Math.max(1, Math.min(numSourceSamples, available));
  const numChannels = Math.max(1, Math.min(2, sourceBuffer.numberOfChannels));

  // Resample to targetSampleRate first — soundtouchjs reads raw channel data
  // and does not respect the source buffer's native sample rate.
  const resampledLength = Math.max(
    1,
    Math.ceil(sourceDuration * targetSampleRate),
  );
  const resampleCtx = new OfflineAudioContext(
    numChannels,
    resampledLength,
    targetSampleRate,
  );
  const nativeBuffer = resampleCtx.createBuffer(
    numChannels,
    actualSamples,
    nativeSampleRate,
  );

  for (let ch = 0; ch < numChannels; ch++) {
    const src = sourceBuffer.getChannelData(
      Math.min(ch, sourceBuffer.numberOfChannels - 1),
    );
    nativeBuffer.copyToChannel(
      src.subarray(startSample, startSample + actualSamples),
      ch,
    );
  }

  const resampleSourceNode = resampleCtx.createBufferSource();
  resampleSourceNode.buffer = nativeBuffer;
  resampleSourceNode.connect(resampleCtx.destination);
  resampleSourceNode.start(0);
  const resampledBuffer = await resampleCtx.startRendering();

  const outputSamples = Math.max(
    1,
    Math.ceil(clipDuration * targetSampleRate),
  );
  const stretchCtx = new OfflineAudioContext(
    numChannels,
    outputSamples,
    targetSampleRate,
  );
  const shifter = new PitchShifter(stretchCtx, resampledBuffer, 4096);
  shifter.tempo = rate;
  shifter.pitch = 1;
  shifter.connect(stretchCtx.destination);
  return stretchCtx.startRendering();
}

export async function renderRetimedBuffer({
  audioContext,
  sourceBuffer,
  trimStart,
  clipDuration,
  retime,
  maintainPitch = false,
}: {
  audioContext: BaseAudioContext;
  sourceBuffer: AudioBuffer;
  trimStart: number;
  clipDuration: number;
  retime?: RetimeConfig;
  maintainPitch?: boolean;
}): Promise<AudioBuffer> {
  const targetSampleRate = audioContext.sampleRate;
  const rate = clampRetimeRate({ rate: retime?.rate ?? 1 });
  const safeTrimStart = sanitizeTrimStart({ trimStart });
  const renderDuration = getRenderableClipDuration({
    sourceDuration: sourceBuffer.duration,
    trimStart: safeTrimStart,
    clipDuration,
    retime,
  });
  const usePitchPreservation =
    shouldMaintainPitch({ rate, maintainPitch }) &&
    Math.abs(rate - 1) > RATE_EPSILON;

  if (usePitchPreservation) {
    return buildPitchPreservedBuffer({
      sourceBuffer,
      trimStart: safeTrimStart,
      clipDuration: renderDuration,
      rate,
      targetSampleRate,
    });
  }

  return buildResampledBuffer({
    audioContext,
    sourceBuffer,
    trimStart: safeTrimStart,
    clipDuration: renderDuration,
    targetSampleRate,
    retime,
  });
}
