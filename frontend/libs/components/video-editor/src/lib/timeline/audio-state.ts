import { isLeafChannelData } from "../animation/channel-data";
import {
  getNormalizedScalarChannelValueAtTime,
  isScalarChannel,
  normalizeScalarChannel,
} from "../animation/interpolation";
import { hasKeyframesForPath } from "../animation/keyframe-query";
import { resolveNumberAtTime } from "../animation/values";
import { VOLUME_DB_MAX, VOLUME_DB_MIN } from "./audio-constants";
import type { TimelineElement } from "./types";
import { TICKS_PER_SECOND } from "../wasm";

// Audio gain resolution: walks volume keyframes + mute flags to compute
// the effective per-frame gain for an audio-capable element. Used by
// the playback engine (live audio) and the waveform renderer (visual
// gain bars match what you'll hear).

const DEFAULT_STEP_SECONDS = 1 / 60;

// Most gain points one buildAudioGainAutomation call produces (10 minutes at
// the default 60 Hz step). Longer spans are sampled with a wider step so a
// long clip from a project document cannot queue millions of ramps.
export const MAX_AUDIO_GAIN_AUTOMATION_POINTS = 36_000;

export type AudioCapableElement = Extract<
  TimelineElement,
  { type: "audio" | "video" }
>;

export function clampDb(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }

  return Math.min(VOLUME_DB_MAX, Math.max(VOLUME_DB_MIN, value));
}

export function dBToLinear(db: number): number {
  return 10 ** (clampDb(db) / 20);
}

export function getElementVolume({
  element,
}: {
  element: AudioCapableElement;
}): number {
  const value = element.params.volume;
  return typeof value === "number" ? value : 0;
}

export function isElementMuted({
  element,
}: {
  element: AudioCapableElement;
}): boolean {
  return element.params.muted === true;
}

export function hasAnimatedVolume({
  element,
}: {
  element: AudioCapableElement;
}): boolean {
  return hasKeyframesForPath({
    animations: element.animations,
    propertyPath: "volume",
  });
}

export function resolveEffectiveAudioGain({
  element,
  trackMuted = false,
  localTime,
}: {
  element: AudioCapableElement;
  trackMuted?: boolean;
  localTime: number;
}): number {
  if (trackMuted || isElementMuted({ element })) {
    return 0;
  }

  const resolvedDb = resolveNumberAtTime({
    baseValue: getElementVolume({ element }),
    animations: element.animations,
    propertyPath: "volume",
    localTime: Math.round(localTime * TICKS_PER_SECOND),
  });

  return dBToLinear(resolvedDb);
}

/**
 * Returns a function equivalent to resolveEffectiveAudioGain for one element
 * at varying local times, with the volume channel normalized once up front.
 * Per-sample callers (export mixing, gain automation) must use this:
 * normalizing per call re-copies and re-sorts every key each time.
 */
export function createAudioGainResolver({
  element,
  trackMuted = false,
}: {
  element: AudioCapableElement;
  trackMuted?: boolean;
}): (localTime: number) => number {
  if (trackMuted || isElementMuted({ element })) {
    return () => 0;
  }

  const data = element.animations?.volume;
  if (
    !isLeafChannelData(data) ||
    !isScalarChannel(data) ||
    data.keys.length === 0
  ) {
    // No keyframed scalar volume: the gain does not vary with time.
    const gain = resolveEffectiveAudioGain({ element, localTime: 0 });
    return () => gain;
  }

  const normalizedChannel = normalizeScalarChannel({ channel: data });
  const baseDb = getElementVolume({ element });
  return (localTime) =>
    dBToLinear(
      getNormalizedScalarChannelValueAtTime({
        normalizedChannel,
        // Same tick rounding and clamping as resolveNumberAtTime.
        time: Math.max(0, Math.round(localTime * TICKS_PER_SECOND)),
        fallbackValue: baseDb,
      }),
    );
}

export function buildWaveformGainSamples({
  element,
  count,
}: {
  element: AudioCapableElement;
  count: number;
}): number[] {
  const durationSeconds = element.duration / TICKS_PER_SECOND;
  const resolveGain = createAudioGainResolver({ element });
  return Array.from({ length: count }, (_, i) => {
    const localTime = ((i + 0.5) / count) * durationSeconds;
    return resolveGain(localTime);
  });
}

export function buildAudioGainAutomation({
  element,
  trackMuted = false,
  fromLocalTime,
  toLocalTime,
  stepSeconds = DEFAULT_STEP_SECONDS,
}: {
  element: AudioCapableElement;
  trackMuted?: boolean;
  fromLocalTime: number;
  toLocalTime: number;
  stepSeconds?: number;
}): Array<{ localTime: number; gain: number }> {
  const startTime = Number.isFinite(fromLocalTime)
    ? Math.max(0, fromLocalTime)
    : 0;
  const endTime = Number.isFinite(toLocalTime)
    ? Math.max(startTime, toLocalTime)
    : startTime;
  const requestedStep =
    Number.isFinite(stepSeconds) && stepSeconds > 0
      ? stepSeconds
      : DEFAULT_STEP_SECONDS;
  const safeStep = Math.max(
    requestedStep,
    (endTime - startTime) / (MAX_AUDIO_GAIN_AUTOMATION_POINTS - 1),
  );
  const resolveGain = createAudioGainResolver({ element, trackMuted });
  const times: number[] = [];

  for (let index = 0; ; index++) {
    const localTime = startTime + index * safeStep;
    if (localTime >= endTime) break;
    times.push(localTime);
  }

  // A widened step could step over a short dip, so keyframe times are always sampled (bounded like the grid).
  if (safeStep > requestedStep) {
    const data = element.animations?.volume;
    if (isLeafChannelData(data) && isScalarChannel(data)) {
      const keyTimes = data.keys
        .map((key) => key.time / TICKS_PER_SECOND)
        .filter((time) => time > startTime && time < endTime)
        .slice(0, MAX_AUDIO_GAIN_AUTOMATION_POINTS);
      times.push(...keyTimes);
      times.sort((left, right) => left - right);
    }
  }

  times.push(endTime);

  return times.map((localTime) => ({ localTime, gain: resolveGain(localTime) }));
}
