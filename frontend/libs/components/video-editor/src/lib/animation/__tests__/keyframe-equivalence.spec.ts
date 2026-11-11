import { describe, expect, test, vi } from "vitest";

import {
  getBezierPoint,
  getDefaultLeftHandle,
  getDefaultRightHandle,
  solveBezierProgressForTime,
} from "../bezier";
import {
  getChannelValueAtTime,
  isScalarChannel,
  normalizeDiscreteChannel,
  normalizeScalarChannel,
} from "../interpolation";
import { getElementKeyframes } from "../keyframe-query";
import {
  removeElementKeyframe,
  removeKeyframe,
  retimeElementKeyframe,
  retimeKeyframe,
  setChannelsInData,
  splitAnimationsAtTime,
} from "../keyframes";
import { getChannelEntriesFromData, isCompositeChannelData } from "../channel-data";
import { formatLinearRgba, type ParamValue } from "../../params";
import { clamp } from "../../utils/math";
import type {
  AnimationChannel,
  ChannelData,
  CompositeChannelData,
  DiscreteAnimationChannel,
  ElementAnimations,
  ScalarAnimationChannel,
  ScalarAnimationKey,
} from "../types";

// Fixed boundary ids so split results can be compared across calls.
vi.mock("../../utils/id", () => ({ generateUUID: () => "boundary" }));

// Differential tests: the linear-time paths must give exactly the results of
// the implementations they replaced. The reference functions below are the
// pre-change algorithms (normalize on every query, scan keys from the start,
// rebuild composite data one component at a time).

describe("channel evaluation matches the pre-change scan", () => {
  test("scalar channels, including duplicate times, bezier and extrapolation", () => {
    const random = createRandom(0x5eed1);
    for (let trial = 0; trial < 400; trial++) {
      const channel = randomScalarChannel(random);
      for (const time of queryTimes(random)) {
        expectSameValue(
          getChannelValueAtTime({ channel, time, fallbackValue: 0 }),
          referenceScalarValueAtTime(channel, time, 0),
        );
      }
    }
  });

  test("discrete channels, including duplicate times and a NaN query time", () => {
    const random = createRandom(0x5eed2);
    for (let trial = 0; trial < 400; trial++) {
      const channel = randomDiscreteChannel(random);
      for (const time of queryTimes(random)) {
        expectSameValue(
          getChannelValueAtTime({ channel, time, fallbackValue: "fallback" }),
          referenceDiscreteValueAtTime(channel, time, "fallback"),
        );
      }
    }
  });

  test("a NaN query time yields the last discrete key, as the scan did", () => {
    const channel: DiscreteAnimationChannel = {
      keys: [
        { id: "a", time: 0 as never, value: "first" },
        { id: "b", time: 10 as never, value: "last" },
      ],
    };
    expect(getChannelValueAtTime({ channel, time: Number.NaN, fallbackValue: "x" })).toBe("last");
  });
});

describe("keyframe listing matches the pre-change evaluation", () => {
  test("leaf, RGBA and mixed composite properties", () => {
    const random = createRandom(0x5eed3);
    for (let trial = 0; trial < 200; trial++) {
      const animations: ElementAnimations = {};
      animations.opacity = randomScalarChannel(random);
      animations["params.label"] = randomDiscreteChannel(random);
      const color: CompositeChannelData = {};
      for (const component of ["r", "g", "b", "a"]) {
        if (random.next() < 0.9) color[component] = randomScalarChannel(random, 4);
      }
      if (random.next() < 0.2) color.extra = randomDiscreteChannel(random, 3);
      if (random.next() < 0.2) color.g = randomDiscreteChannel(random, 3);
      animations.color = color;

      const listed = getElementKeyframes({ animations });
      const expected: Array<{ propertyPath: string; time: number; value: ParamValue }> = [];
      for (const [propertyPath, data] of Object.entries(animations)) {
        if (!data) continue;
        const times = [
          ...new Set(
            getChannelEntriesFromData({ data }).flatMap(([, channel]) =>
              channel.keys.map((key) => key.time as number),
            ),
          ),
        ].sort((left, right) => left - right);
        for (const time of times) {
          const value = referenceComposedValue(data, time);
          if (value !== null) expected.push({ propertyPath, time, value });
        }
      }
      expect(
        listed.map(({ propertyPath, time, value }) => ({ propertyPath, time, value })),
      ).toEqual(expected);
    }
  });
});

describe("batched composite updates match chained single updates", () => {
  const randomChannel = (random: Random): AnimationChannel | undefined => {
    const roll = random.next();
    if (roll < 0.2) return undefined;
    if (roll < 0.35) return { keys: [] } as ScalarAnimationChannel;
    return roll < 0.7 ? randomScalarChannel(random, 3) : randomDiscreteChannel(random, 3);
  };

  test("setChannelsInData over leaf, composite and empty data", () => {
    const random = createRandom(0x5eed4);
    for (let trial = 0; trial < 2_000; trial++) {
      let data: ChannelData | undefined;
      const shape = random.int(0, 2);
      if (shape === 1) data = randomScalarChannel(random, 3);
      if (shape === 2) {
        const composite: CompositeChannelData = {};
        for (const key of ["x", "y", "z"]) {
          if (random.next() < 0.6) composite[key] = randomScalarChannel(random, 3);
        }
        data = composite;
      }
      const updates: Array<[string, AnimationChannel | undefined]> = Array.from(
        { length: random.int(0, 6) },
        () => [random.pick(["value", "x", "y", "z", "w"]), randomChannel(random)],
      );

      let expected = data;
      for (const [componentKey, channel] of updates) {
        expected = referenceSetChannelInData(expected, componentKey, channel);
      }
      const actual = setChannelsInData({ data, updates });

      expect(actual).toStrictEqual(expected);
      if (updates.length === 0) expect(actual).toBe(data);
      if (isCompositeChannelData(actual)) {
        expect(Object.keys(actual)).toEqual(Object.keys(expected as CompositeChannelData));
      }
    }
  });

  test("splitAnimationsAtTime on composite data", () => {
    const random = createRandom(0x5eed6);
    for (let trial = 0; trial < 300; trial++) {
      const data: CompositeChannelData = {};
      const componentCount = random.int(1, 5);
      for (let i = 0; i < componentCount; i++) {
        data[`c${i}`] =
          random.next() < 0.7 ? randomScalarChannel(random, 4) : randomDiscreteChannel(random, 4);
      }
      const splitTime = random.int(-1, 13) as never;
      const shouldIncludeSplitBoundary = random.next() < 0.5;
      const actual = splitAnimationsAtTime({
        animations: { "params.many": data },
        splitTime,
        shouldIncludeSplitBoundary,
      });

      // Each component split on its own, then folded in one at a time as
      // before the change.
      let expectedLeft: ChannelData | undefined;
      let expectedRight: ChannelData | undefined;
      for (const [componentKey, channel] of Object.entries(data)) {
        const single = splitAnimationsAtTime({
          animations: { "params.one": channel },
          splitTime,
          shouldIncludeSplitBoundary,
        });
        const left = single.leftAnimations?.["params.one"] as AnimationChannel | undefined;
        const right = single.rightAnimations?.["params.one"] as AnimationChannel | undefined;
        if (left) expectedLeft = referenceSetChannelInData(expectedLeft, componentKey, left);
        if (right) expectedRight = referenceSetChannelInData(expectedRight, componentKey, right);
      }

      expect(actual.leftAnimations?.["params.many"]).toStrictEqual(expectedLeft);
      expect(actual.rightAnimations?.["params.many"]).toStrictEqual(expectedRight);
    }
  });

  test("removeElementKeyframe and retimeElementKeyframe on composite data", () => {
    const random = createRandom(0x5eed5);
    for (let trial = 0; trial < 300; trial++) {
      const data: CompositeChannelData = {};
      const componentCount = random.int(1, 6);
      for (let i = 0; i < componentCount; i++) {
        const channel = randomScalarChannel(random, 4);
        if (random.next() < 0.5) channel.keys[0].id = "shared";
        data[`c${i}`] = channel;
      }
      const animations: ElementAnimations = { "params.many": data };
      const keyframeId = random.next() < 0.8 ? "shared" : "missing";

      let expectedRemoved: ChannelData | undefined = data;
      let expectedRetimed: ChannelData | undefined = data;
      for (const [componentKey, channel] of Object.entries(data)) {
        expectedRemoved = referenceSetChannelInData(
          expectedRemoved,
          componentKey,
          removeKeyframe({ channel, keyframeId }),
        );
        expectedRetimed = referenceSetChannelInData(
          expectedRetimed,
          componentKey,
          retimeKeyframe({ channel, keyframeId, time: 7 as never }),
        );
      }

      expect(
        removeElementKeyframe({ animations, propertyPath: "params.many", keyframeId })?.[
          "params.many"
        ],
      ).toStrictEqual(expectedRemoved);
      expect(
        retimeElementKeyframe({
          animations,
          propertyPath: "params.many",
          keyframeId,
          time: 7 as never,
        })?.["params.many"],
      ).toStrictEqual(expectedRetimed);
    }
  });
});

function referenceScalarValueAtTime(
  channel: ScalarAnimationChannel,
  time: number,
  fallbackValue: number,
): number {
  if (channel.keys.length === 0) return fallbackValue;
  const normalized = normalizeScalarChannel({ channel });
  const keys = normalized.keys;
  const firstKey = keys[0];
  const lastKey = keys[keys.length - 1];
  const extrapolate = (
    mode: "hold" | "linear",
    edgeKey: ScalarAnimationKey,
    neighborKey: ScalarAnimationKey | undefined,
  ) => {
    if (mode === "hold" || !neighborKey) return edgeKey.value;
    const span = neighborKey.time - edgeKey.time;
    if (span === 0) return edgeKey.value;
    return edgeKey.value + ((time - edgeKey.time) / span) * (neighborKey.value - edgeKey.value);
  };
  if (time <= firstKey.time) {
    if (time < firstKey.time) {
      return extrapolate(normalized.extrapolation?.before ?? "hold", firstKey, keys[1]);
    }
    return firstKey.value;
  }
  if (time >= lastKey.time) {
    if (time > lastKey.time) {
      return extrapolate(normalized.extrapolation?.after ?? "hold", lastKey, keys[keys.length - 2]);
    }
    return lastKey.value;
  }
  for (let keyIndex = 0; keyIndex < keys.length - 1; keyIndex++) {
    const leftKey = keys[keyIndex];
    const rightKey = keys[keyIndex + 1];
    if (time === rightKey.time) return rightKey.value;
    if (!(time >= leftKey.time && time <= rightKey.time)) continue;
    if (leftKey.segmentToNext === "step") return leftKey.value;
    const span = rightKey.time - leftKey.time;
    if (span === 0) return rightKey.value;
    const progress = clamp({ value: (time - leftKey.time) / span, min: 0, max: 1 });
    if (leftKey.segmentToNext === "linear") {
      return leftKey.value + (rightKey.value - leftKey.value) * progress;
    }
    const curveProgress = solveBezierProgressForTime({ time, leftKey, rightKey });
    const rightHandle = leftKey.rightHandle ?? getDefaultRightHandle({ leftKey, rightKey });
    const leftHandle = rightKey.leftHandle ?? getDefaultLeftHandle({ leftKey, rightKey });
    return getBezierPoint({
      progress: curveProgress,
      p0: leftKey.value,
      p1: leftKey.value + rightHandle.dv,
      p2: rightKey.value + leftHandle.dv,
      p3: rightKey.value,
    });
  }
  return lastKey.value;
}

function referenceDiscreteValueAtTime(
  channel: DiscreteAnimationChannel,
  time: number,
  fallbackValue: string | boolean,
): string | boolean {
  if (channel.keys.length === 0) return fallbackValue;
  let currentValue = fallbackValue;
  for (const key of normalizeDiscreteChannel({ channel }).keys) {
    if (time < key.time) break;
    currentValue = key.value;
  }
  return currentValue;
}

function referenceChannelValue(channel: AnimationChannel, time: number): ParamValue {
  const fallbackValue =
    channel.keys.length === 0 ? (isScalarChannel(channel) ? 0 : false) : channel.keys[0].value;
  if (channel.keys.length === 0) return fallbackValue;
  if (typeof fallbackValue === "number") {
    return isScalarChannel(channel)
      ? referenceScalarValueAtTime(channel, time, fallbackValue)
      : fallbackValue;
  }
  if (isScalarChannel(channel)) return fallbackValue;
  return referenceDiscreteValueAtTime(channel, time, fallbackValue);
}

function referenceComposedValue(data: ChannelData, time: number): ParamValue | null {
  const entries = getChannelEntriesFromData({ data });
  if (entries.length === 0) return null;
  if (entries.length === 1 && entries[0][0] === "value") {
    return referenceChannelValue(entries[0][1], time);
  }
  const values = Object.fromEntries(
    entries.map(([componentKey, channel]) => [componentKey, referenceChannelValue(channel, time)]),
  );
  if (
    typeof values.r === "number" &&
    typeof values.g === "number" &&
    typeof values.b === "number" &&
    typeof values.a === "number"
  ) {
    return formatLinearRgba({ color: { r: values.r, g: values.g, b: values.b, a: values.a } });
  }
  return null;
}

function referenceSetChannelInData(
  data: ChannelData | undefined,
  componentKey: string,
  channel: AnimationChannel | undefined,
): ChannelData | undefined {
  if (componentKey === "value") return channel;
  const components: CompositeChannelData = isCompositeChannelData(data) ? { ...data } : {};
  if (channel && channel.keys.length > 0) {
    components[componentKey] = channel;
  } else {
    delete components[componentKey];
  }
  return Object.keys(components).length > 0 ? components : undefined;
}

// Deterministic PRNG (mulberry32) so failures reproduce.
function createRandom(seed: number) {
  let state = seed >>> 0;
  const next = () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const int = (min: number, max: number) => min + Math.floor(next() * (max - min + 1));
  const pick = <T,>(items: readonly T[]): T => items[int(0, items.length - 1)];
  return { next, int, pick };
}

type Random = ReturnType<typeof createRandom>;

let idCounter = 0;

function randomScalarChannel(random: Random, maxKeys = 8): ScalarAnimationChannel {
  const keys = Array.from({ length: random.int(1, maxKeys) }, () => {
    const key: ScalarAnimationKey = {
      id: `s${idCounter++}`,
      // A small range so duplicate times are common.
      time: random.int(0, 12) as never,
      value: Math.round((random.next() * 2 - 1) * 1000) / 100,
      segmentToNext: random.pick(["step", "linear", "bezier"] as const),
      tangentMode: random.pick(["auto", "aligned", "broken", "flat"] as const),
    };
    if (random.next() < 0.5) {
      key.rightHandle = { dt: random.int(-3, 8) as never, dv: random.next() * 4 - 2 };
    }
    if (random.next() < 0.5) {
      key.leftHandle = { dt: random.int(-8, 3) as never, dv: random.next() * 4 - 2 };
    }
    return key;
  });
  const channel: ScalarAnimationChannel = { keys };
  if (random.next() < 0.5) {
    channel.extrapolation = {
      before: random.pick(["hold", "linear"] as const),
      after: random.pick(["hold", "linear"] as const),
    };
  }
  return channel;
}

function randomDiscreteChannel(random: Random, maxKeys = 8): DiscreteAnimationChannel {
  const asBoolean = random.next() < 0.3;
  return {
    keys: Array.from({ length: random.int(1, maxKeys) }, () => ({
      id: `d${idCounter++}`,
      time: random.int(0, 12) as never,
      value: asBoolean ? random.next() < 0.5 : random.pick(["a", "b", "c", "d"]),
    })),
  };
}

function queryTimes(random: Random): number[] {
  const times = [Number.NaN, -1, -0.5, 0, 6.5, 12, 12.5, 20];
  for (let i = 0; i < 12; i++) times.push(random.int(-2, 14), random.next() * 16 - 2);
  return times;
}

function expectSameValue(actual: unknown, expected: unknown) {
  // Object.is so NaN results and -0/+0 must match exactly too.
  expect(Object.is(actual, expected)).toBe(true);
}
