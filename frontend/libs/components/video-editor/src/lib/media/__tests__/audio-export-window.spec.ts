import { describe, expect, test, vi } from "vitest";

import { createTimelineAudioBuffer } from "../audio";
import type { LibraryAudioElement, SceneTracks } from "../../timeline/types";
import { TICKS_PER_SECOND } from "../../wasm";

const rendered = vi.hoisted(() => vi.fn());

vi.mock("../audio-mastering", () => ({
  applyAudioMasteringToBuffer: async ({ audioBuffer }: { audioBuffer: unknown }) => audioBuffer,
}));
vi.mock("../../retime/audio-stretch", () => ({
  renderRetimedBuffer: async (args: { clipDuration: number }) => {
    rendered(args);
    return fakeBuffer([new Float32Array(Math.ceil(args.clipDuration * 100)).fill(0.5)], 100);
  },
}));

const ticks = (seconds: number) => (seconds * TICKS_PER_SECOND) as never;

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
    fakeBuffer(Array.from({ length: count }, () => new Float32Array(length)), sampleRate),
} as unknown as AudioContext;

function slowClip(overrides: Record<string, unknown>): LibraryAudioElement {
  return {
    id: "c",
    name: "c",
    type: "audio",
    sourceType: "library",
    sourceUrl: "x",
    startTime: ticks(0),
    duration: ticks(3_600),
    trimStart: ticks(0),
    trimEnd: ticks(0),
    params: { volume: 0 },
    retime: { rate: 0.5, maintainPitch: true },
    buffer: fakeBuffer([new Float32Array(1_000)], 100),
    ...overrides,
  } as unknown as LibraryAudioElement;
}

const tracksWith = (elements: LibraryAudioElement[]) =>
  ({
    overlay: [],
    main: { id: "main", name: "main", type: "video", elements: [], muted: false, hidden: false },
    audio: [{ id: "a1", name: "audio", type: "audio", elements, muted: false }],
  }) as unknown as SceneTracks;

describe("pitch-preserving export preparation", () => {
  test("prepares only the exported interval and keeps clip timing", async () => {
    rendered.mockClear();
    // clip starts 1s before 0; the 2s export shows clip time 1s..3s
    const output = await createTimelineAudioBuffer({
      tracks: tracksWith([slowClip({ startTime: ticks(-1), trimStart: ticks(2) })]),
      mediaAssets: [],
      duration: ticks(2),
      sampleRate: 100,
      audioContext: fakeContext,
    });
    expect(rendered).toHaveBeenCalledTimes(1);
    const args = rendered.mock.calls[0][0];
    expect(args.clipDuration).toBeCloseTo(2);
    expect(args.trimStart).toBeCloseTo(2 + 1 * 0.5);
    const left = output!.getChannelData(0);
    expect(left[0]).toBeCloseTo(0.5);
    expect(left[199]).toBeCloseTo(0.5);
  });

  test("skips preparation for a clip outside the export", async () => {
    rendered.mockClear();
    await createTimelineAudioBuffer({
      tracks: tracksWith([slowClip({ startTime: ticks(10) })]),
      mediaAssets: [],
      duration: ticks(2),
      sampleRate: 100,
      audioContext: fakeContext,
    });
    expect(rendered).not.toHaveBeenCalled();
  });
});
