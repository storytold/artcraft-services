import { describe, expect, it } from "vitest";
import { Bone, Group, Object3D } from "three";
import { BoneJSONHelper, MAX_POSE_BONES, MAX_POSE_DEPTH, parseBonePose } from "./BoneJSONHelper";

function makeCharacter(): { model: Object3D; hip: Bone; spine: Bone } {
  const model = new Group();
  model.userData.isCharacter = true;
  const armature = new Group();
  const hip = new Bone();
  hip.name = "mixamorigHips";
  const spine = new Bone();
  spine.name = "mixamorigSpine";
  hip.add(spine);
  armature.add(hip);
  model.add(armature);
  return { model, hip, spine };
}

describe("BoneJSONHelper", () => {
  it("round-trips a pose through toJSON", () => {
    const source = makeCharacter();
    source.hip.position.set(1, 2, 3);
    source.spine.rotation.set(0.1, 0.2, 0.3);
    source.spine.scale.set(1, 2, 0.5);
    source.hip.updateMatrixWorld(true);
    const json = JSON.parse(JSON.stringify(new BoneJSONHelper(source.model).toJSON()));

    const target = makeCharacter();
    new BoneJSONHelper(target.model).poseFromBoneJSON(json);

    expect(target.hip.position.toArray()).toEqual([1, 2, 3]);
    expect(target.spine.quaternion.angleTo(source.spine.quaternion)).toBeLessThan(1e-6);
    expect(target.spine.scale.distanceTo(source.spine.scale)).toBeLessThan(1e-6);
  });

  it("ignores non-bone sections such as procedural geometry", () => {
    const target = makeCharacter();
    const hostile = {
      metadata: { version: 4.6, type: "Object" },
      geometries: [
        { uuid: "g", type: "SphereGeometry", radius: 1, widthSegments: 1e6, heightSegments: 1e6 },
      ],
      object: {
        type: "Mesh",
        geometry: "g",
        matrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 4, 5, 6, 1],
        children: [],
      },
    };
    new BoneJSONHelper(target.model).poseFromBoneJSON(hostile);
    expect(target.hip.position.toArray()).toEqual([4, 5, 6]);
  });

  it("caps the number of nodes read", () => {
    const wide = {
      object: {
        matrix: undefined,
        children: Array.from({ length: MAX_POSE_BONES * 4 }, () => ({ children: [] })),
      },
    };
    const tree = parseBonePose(wide);
    expect(tree?.children.length).toBe(MAX_POSE_BONES - 1);

    let deep: unknown = { children: [] };
    for (let i = 0; i < 10_000; i++) deep = { children: [deep] };
    expect(() => parseBonePose({ object: deep })).not.toThrow();

    let chain: unknown = { children: [] };
    for (let i = 0; i < MAX_POSE_DEPTH + 10; i++) chain = { children: [chain] };
    let levels = 0;
    for (let node = parseBonePose({ object: chain }); node; node = node.children[0]) levels++;
    expect(levels).toBe(MAX_POSE_DEPTH + 1);
  });

  it("keeps sibling indexes when a child entry is unreadable", () => {
    const tree = parseBonePose({
      object: { children: [null, { position: [7, 8, 9], children: [] }] },
    });
    expect(tree?.children.length).toBe(2);
    expect(tree?.children[0]).toBeUndefined();
    expect(tree?.children[1]?.position.toArray()).toEqual([7, 8, 9]);
  });

  it("falls back to identity for a degenerate matrix", () => {
    const tree = parseBonePose({
      object: { matrix: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 1], children: [] },
    });
    expect(tree?.position.toArray()).toEqual([0, 0, 0]);
    expect(tree?.quaternion.toArray()).toEqual([0, 0, 0, 1]);
    expect(tree?.scale.toArray()).toEqual([1, 1, 1]);
  });

  it("returns nothing for documents without an object root", () => {
    expect(parseBonePose(null)).toBeUndefined();
    expect(parseBonePose({ geometries: [] })).toBeUndefined();
  });

  it("rejects an array root and reports truncation", () => {
    expect(parseBonePose({ object: [] })).toBeUndefined();
    const report = { truncated: false };
    parseBonePose({ object: { children: Array.from({ length: MAX_POSE_BONES + 1 }, () => ({})) } }, report);
    expect(report.truncated).toBe(true);
    const ok = { truncated: false };
    parseBonePose({ object: { children: [{}] } }, ok);
    expect(ok.truncated).toBe(false);
  });

  it("skips a truncated pose instead of applying part of it", () => {
    const target = makeCharacter();
    new BoneJSONHelper(target.model).poseFromBoneJSON({
      object: { position: [5, 5, 5], children: Array.from({ length: MAX_POSE_BONES + 1 }, () => ({})) },
    });
    expect(target.hip.position.toArray()).toEqual([0, 0, 0]);
  });
});
