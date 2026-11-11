import { afterEach, describe, expect, test, vi } from "vitest";

import {
  collectAudioClips,
  createTimelineAudioBuffer,
  MAX_TIMELINE_AUDIO_SECONDS,
} from "../audio";
import {
  getRenderableClipDuration,
  MAX_RETIMED_CLIP_SECONDS,
} from "../../retime/audio-stretch";
import {
  buildAudioGainAutomation,
  MAX_AUDIO_GAIN_AUTOMATION_POINTS,
  resolveEffectiveAudioGain,
  type AudioCapableElement,
} from "../../timeline/audio-state";
import type {
  LibraryAudioElement,
  SceneTracks,
} from "../../timeline/types";
import type { ElementAnimations } from "../../animation/types";
import { TICKS_PER_SECOND } from "../../wasm";

const ticks = (seconds: number) => (seconds * TICKS_PER_SECOND) as never;

function libraryAudio(
  overrides: Partial<LibraryAudioElement> = {},
): LibraryAudioElement {
  return {
    id: "clip-1",
    name: "clip",
    type: "audio",
    sourceType: "library",
    sourceUrl: "https://cdn.example/clip.mp3",
    startTime: ticks(2),
    duration: ticks(3),
    trimStart: ticks(0.5),
    trimEnd: ticks(0),
    params: { volume: 0 },
    ...overrides,
  } as LibraryAudioElement;
}

function tracksWith(elements: LibraryAudioElement[]): SceneTracks {
  return {
    overlay: [],
    main: { id: "main", name: "main", type: "video", elements: [], muted: false, hidden: false },
    audio: [{ id: "a1", name: "audio", type: "audio", elements, muted: false }],
  } as SceneTracks;
}

// Mastering needs a real OfflineAudioContext; the mix itself is under test.
vi.mock("../audio-mastering", () => ({
  applyAudioMasteringToBuffer: async ({ audioBuffer }: { audioBuffer: unknown }) =>
    audioBuffer,
}));

afterEach(() => {
  vi.unstubAllGlobals();
});

function fakeBuffer(channels: Float32Array[], sampleRate: number): AudioBuffer {
  return {
    numberOfChannels: channels.length,
    length: channels[0].length,
    sampleRate,
    duration: channels[0].length / sampleRate,
    getChannelData: (channel: number) => channels[channel],
  } as unknown as AudioBuffer;
}

const fakeContext = {
  createBuffer: (count: number, length: number, sampleRate: number) =>
    fakeBuffer(
      Array.from({ length: count }, () => new Float32Array(length)),
      sampleRate,
    ),
} as unknown as AudioContext;

describe("getRenderableClipDuration", () => {
  test("stops where the source audio runs out", () => {
    expect(
      getRenderableClipDuration({
        sourceDuration: 5,
        trimStart: 1,
        clipDuration: 36_000,
      }),
    ).toBe(4);
    expect(
      getRenderableClipDuration({
        sourceDuration: 5,
        trimStart: 1,
        clipDuration: 10,
        retime: { rate: 2 },
      }),
    ).toBe(2);
  });

  test("caps slowed-down clips at the prepared-buffer limit", () => {
    expect(
      getRenderableClipDuration({
        sourceDuration: 3_600,
        trimStart: 0,
        clipDuration: 1e9,
        retime: { rate: 0.01 },
      }),
    ).toBe(MAX_RETIMED_CLIP_SECONDS);
  });

  test("returns 0 for invalid or exhausted spans", () => {
    for (const clipDuration of [Number.NaN, Infinity, -5, 0]) {
      expect(
        getRenderableClipDuration({ sourceDuration: 5, trimStart: 0, clipDuration }),
      ).toBe(0);
    }
    expect(
      getRenderableClipDuration({ sourceDuration: 5, trimStart: 9, clipDuration: 3 }),
    ).toBe(0);
  });
});

describe("createTimelineAudioBuffer", () => {
  test("rejects a timeline longer than the export budget before decoding", async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);

    await expect(
      createTimelineAudioBuffer({
        tracks: tracksWith([libraryAudio()]),
        mediaAssets: [],
        duration: ticks(MAX_TIMELINE_AUDIO_SECONDS + 1),
      }),
    ).rejects.toThrow(/too long/);
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});

describe("mixAudioChannels", () => {
  const sampleRate = 100;
  const source = fakeBuffer([new Float32Array(1_000).fill(0.5)], sampleRate);

  test("mixes only the part of a clip that overlaps the output", async () => {
    // Starts ~11.5 days before 0 with a matching negative trim, so the old
    // loop walked 1e8 leading samples that never reached the output.
    const element = libraryAudio({
      buffer: source,
      startTime: ticks(-1_000_000),
      duration: ticks(1_000_000 + 2),
      trimStart: ticks(-1_000_000),
    });

    const started = performance.now();
    const output = await createTimelineAudioBuffer({
      tracks: tracksWith([element]),
      mediaAssets: [],
      duration: ticks(4),
      sampleRate,
      audioContext: fakeContext,
    });
    expect(performance.now() - started).toBeLessThan(1_000);

    // Source (10s) is exhausted long before the clip reaches the output.
    expect(Array.from(output!.getChannelData(0)).every((v) => v === 0)).toBe(true);
  });

  test("ignores a negative trim instead of reading before the source", async () => {
    const element = libraryAudio({
      buffer: source,
      startTime: ticks(1),
      duration: ticks(1),
      trimStart: ticks(-5),
    });

    const output = await createTimelineAudioBuffer({
      tracks: tracksWith([element]),
      mediaAssets: [],
      duration: ticks(3),
      sampleRate,
      audioContext: fakeContext,
    });

    const left = output!.getChannelData(0);
    const right = output!.getChannelData(1);
    expect(Array.from(left).some(Number.isNaN)).toBe(false);
    expect(left[150]).toBeCloseTo(0.5);
    expect(right[150]).toBeCloseTo(0.5);
    expect(left[50]).toBe(0);
    expect(left[250]).toBe(0);
  });
});

describe("collectAudioClips", () => {
  test("reports library clip timing in seconds, like uploaded clips", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(new Blob([new Uint8Array(4)]))),
    );

    const [clip] = await collectAudioClips({
      tracks: tracksWith([libraryAudio()]),
      mediaAssets: [],
    });

    expect(clip.startTime).toBeCloseTo(2);
    expect(clip.duration).toBeCloseTo(3);
    expect(clip.trimStart).toBeCloseTo(0.5);
  });
});

describe("buildAudioGainAutomation", () => {
  const animated = libraryAudio({
    duration: ticks(10 * 3_600),
    animations: {
      volume: {
        keys: [0, 1, 2, 5].map((seconds, index) => ({
          id: `k${index}`,
          time: ticks(seconds),
          value: index % 2 === 0 ? 0 : -12,
          segmentToNext: "linear" as const,
          tangentMode: "flat" as const,
        })),
      },
    } as ElementAnimations,
  }) as AudioCapableElement;

  test("matches per-call gain resolution", () => {
    const points = buildAudioGainAutomation({
      element: animated,
      fromLocalTime: 0,
      toLocalTime: 6,
    });
    for (const point of points.filter((_, index) => index % 17 === 0)) {
      expect(point.gain).toBeCloseTo(
        resolveEffectiveAudioGain({ element: animated, localTime: point.localTime }),
        10,
      );
    }
  });

  test("caps the number of points for a very long clip", () => {
    const points = buildAudioGainAutomation({
      element: animated,
      fromLocalTime: 0,
      toLocalTime: 10 * 3_600,
    });
    expect(points.length).toBeLessThanOrEqual(MAX_AUDIO_GAIN_AUTOMATION_POINTS + 1);
    expect(points[points.length - 1].localTime).toBe(10 * 3_600);
  });
});
