import { describe, expect, it } from "vitest";
import { clampSampleIndex, computeRmsBuckets } from "../waveform-summary";

// Minimal AudioBuffer stand-in: one channel of constant 0.5 samples.
const fakeBuffer = (length: number) =>
  ({
    numberOfChannels: 1,
    sampleRate: 48_000,
    length,
    getChannelData: () => new Float32Array(length).fill(0.5),
  }) as unknown as AudioBuffer;

describe("waveform analysis ranges", () => {
  it("clamps sample indexes into the buffer", () => {
    expect(clampSampleIndex(-10, 100)).toBe(0);
    expect(clampSampleIndex(1e12, 100)).toBe(100);
    expect(clampSampleIndex(Number.NaN, 100)).toBe(0);
    expect(clampSampleIndex(Number.POSITIVE_INFINITY, 100)).toBe(0);
    expect(clampSampleIndex(7.9, 100)).toBe(7);
  });

  it("never scans past the buffer for hostile bucket endpoints", () => {
    const buffer = fakeBuffer(1_000);
    const started = Date.now();
    const result = computeRmsBuckets({
      buffer,
      buckets: [
        { bucketStart: -5, bucketEnd: 10 },
        { bucketStart: 0, bucketEnd: 2 ** 40 },
        { bucketStart: 2 ** 40, bucketEnd: 2 ** 41 },
        { bucketStart: Number.NaN, bucketEnd: Number.POSITIVE_INFINITY },
      ],
    });
    expect(result).toHaveLength(4);
    expect(result[0]).toBeCloseTo(0.5);
    expect(result[1]).toBeCloseTo(0.5);
    expect(result[2]).toBe(0);
    expect(result[3]).toBe(0);
    expect(Date.now() - started).toBeLessThan(2_000);
  });
});
