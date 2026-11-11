import { describe, expect, it } from "vitest";
import type Editor from "../editor";
import { TimelineController } from "./TimelineController";
import {
  DEFAULT_TIMELINE_DURATION,
  DEFAULT_TIMELINE_FPS,
  MAX_TIMELINE_DURATION,
  clampTimelineDuration,
  type TimelineData,
} from "../timeline/types";

function makeController(): TimelineController {
  const editor = {
    bus: { emit: () => undefined },
    characterAnimationManager: { sync: () => undefined, evaluateAt: () => undefined },
    cameraController: {},
  } as unknown as Editor;
  return new TimelineController(editor);
}

function load(duration: unknown, fps: unknown = 30): TimelineData | null {
  const controller = makeController();
  controller.loadTimeline({
    duration,
    fps,
    tracks: [],
    clipLanes: [],
  } as unknown as TimelineData);
  return controller.getTimeline();
}

describe("timeline duration bounds", () => {
  it("clamps a loaded duration into the supported range", () => {
    expect(load(1e9)?.duration).toBe(MAX_TIMELINE_DURATION);
    expect(load(Number.POSITIVE_INFINITY)?.duration).toBe(DEFAULT_TIMELINE_DURATION);
    expect(load(Number.NaN)?.duration).toBe(DEFAULT_TIMELINE_DURATION);
    expect(load("1000000")?.duration).toBe(DEFAULT_TIMELINE_DURATION);
    expect(load(-5)?.duration).toBe(DEFAULT_TIMELINE_DURATION);
    expect(load(0.25)?.duration).toBe(1);
    expect(load(24)?.duration).toBe(24);
  });

  it("falls back to the default fps for non-finite values", () => {
    expect(load(10, Number.POSITIVE_INFINITY)?.fps).toBe(DEFAULT_TIMELINE_FPS);
  });

  it("clampTimelineDuration leaves supported values unchanged", () => {
    expect(clampTimelineDuration(MAX_TIMELINE_DURATION)).toBe(MAX_TIMELINE_DURATION);
    expect(clampTimelineDuration(5)).toBe(5);
  });
});
