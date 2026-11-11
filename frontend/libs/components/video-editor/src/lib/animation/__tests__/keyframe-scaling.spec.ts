import { describe, expect, test } from "vitest";

import { getChannelValueAtTime } from "../interpolation";
import { getElementKeyframes, getKeyframeAtTime } from "../keyframe-query";
import { removeElementKeyframe, retimeElementKeyframe } from "../keyframes";
import type {
  CompositeChannelData,
  DiscreteAnimationChannel,
  ElementAnimations,
  ScalarAnimationChannel,
} from "../types";

describe("keyframe listing", () => {
  test("lists scalar keyframes with the same values as direct evaluation", () => {
    const channel = scalarChannel([0, 10, 5, 20, 15], (t) => t * 2);
    const keyframes = getElementKeyframes({ animations: { opacity: channel } as ElementAnimations });

    expect(keyframes.map((k) => k.time)).toEqual([0, 5, 10, 15, 20]);
    for (const keyframe of keyframes) {
      expect(keyframe.value).toBe(
        getChannelValueAtTime({ channel, time: keyframe.time, fallbackValue: 0 }),
      );
    }
  });

  test("evaluates every color component at each listed time", () => {
    const color: CompositeChannelData = {
      r: scalarChannel([0, 10], (t) => t / 10),
      g: scalarChannel([0, 10], () => 0.5),
      b: scalarChannel([5], () => 0.25),
      a: scalarChannel([0], () => 1),
    };
    const animations = { color } as ElementAnimations;
    const listed = getElementKeyframes({ animations });

    expect(listed.map((k) => k.time)).toEqual([0, 5, 10]);
    for (const keyframe of listed) {
      expect(keyframe.value).toEqual(
        getKeyframeAtTime({ animations, propertyPath: "color", time: keyframe.time })?.value,
      );
    }
  });

  test("evaluates discrete channels at their keys", () => {
    const channel: DiscreteAnimationChannel = {
      keys: [
        { id: "b", time: 10 as never, value: "second" },
        { id: "a", time: 0 as never, value: "first" },
      ],
    };
    const listed = getElementKeyframes({
      animations: { "params.label": channel } as ElementAnimations,
    });
    expect(listed.map((k) => k.value)).toEqual(["first", "second"]);
  });

  test("lists a long channel without re-normalizing it per key", () => {
    const times = Array.from({ length: 20_000 }, (_, i) => 20_000 - i);
    const channel = scalarChannel(times, (t) => t);
    const started = performance.now();
    const listed = getElementKeyframes({ animations: { opacity: channel } as ElementAnimations });
    expect(listed).toHaveLength(times.length);
    expect(listed[123].value).toBe(listed[123].time);
    // Quadratic re-normalization took minutes at this size.
    expect(performance.now() - started).toBeLessThan(5_000);
  });
});

describe("composite channel updates", () => {
  test("removes and retimes a keyframe across many components in one pass", () => {
    const data: CompositeChannelData = {};
    for (let i = 0; i < 2_000; i++) {
      data[`c${i}`] = {
        keys: [
          { ...scalarKey(0, i), id: "shared" },
          { ...scalarKey(10, i), id: `own-${i}` },
        ],
      };
    }
    data.c0 = { keys: [{ ...scalarKey(0, 0), id: "shared" }] };
    const animations = { "params.many": data } as ElementAnimations;

    const removed = removeElementKeyframe({
      animations,
      propertyPath: "params.many",
      keyframeId: "shared",
    })?.["params.many"] as CompositeChannelData;
    expect(removed.c0).toBeUndefined();
    expect(Object.keys(removed)).toHaveLength(1_999);
    expect(removed.c5?.keys.map((k) => k.id)).toEqual(["own-5"]);

    const retimed = retimeElementKeyframe({
      animations,
      propertyPath: "params.many",
      keyframeId: "shared",
      time: 4 as never,
    })?.["params.many"] as CompositeChannelData;
    expect(Object.keys(retimed)).toHaveLength(2_000);
    expect(retimed.c7?.keys.find((k) => k.id === "shared")?.time).toBe(4);
  });
});

function scalarKey(time: number, value: number) {
  return {
    id: `k${time}`,
    time: time as never,
    value,
    segmentToNext: "linear" as const,
    tangentMode: "flat" as const,
  };
}

function scalarChannel(times: number[], valueAt: (time: number) => number): ScalarAnimationChannel {
  return { keys: times.map((time) => scalarKey(time, valueAt(time))) };
}
