import { describe, expect, it } from "vitest";
import {
  MAX_RECORD_FPS,
  MAX_RECORD_SECONDS,
  recordingFrameBudget,
} from "./TimelineRecorder";

describe("recordingFrameBudget", () => {
  it("keeps supported timelines as they are", () => {
    expect(recordingFrameBudget({ fps: 30, duration: 5 })).toEqual({
      fps: 30,
      frameCount: 150,
    });
  });

  it("caps imported duration and fps", () => {
    expect(recordingFrameBudget({ fps: 1e6, duration: 1e9 })).toEqual({
      fps: MAX_RECORD_FPS,
      frameCount: MAX_RECORD_SECONDS * MAX_RECORD_FPS,
    });
    expect(recordingFrameBudget({ fps: Number.NaN, duration: -1 })).toEqual({
      fps: 30,
      frameCount: 1,
    });
  });
});
