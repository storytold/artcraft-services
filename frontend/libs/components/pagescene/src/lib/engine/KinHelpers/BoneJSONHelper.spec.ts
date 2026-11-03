import { describe, expect, it } from "vitest";
import { Bone, Group, Object3D } from "three";
import { BoneJSONHelper, MAX_POSE_BONES, parseBonePose } from "./BoneJSONHelper";

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
    source.hip.updateMatrixWorld(true);
    const json = JSON.parse(JSON.stringify(new BoneJSONHelper(source.model).toJSON()));

    const target = makeCharacter();
    new BoneJSONHelper(target.model).poseFromBoneJSON(json);

    expect(target.hip.position.toArray()).toEqual([1, 2, 3]);
    expect(target.spine.quaternion.angleTo(source.spine.quaternion)).toBeLessThan(1e-6);
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
  });

  it("returns nothing for documents without an object root", () => {
    expect(parseBonePose(null)).toBeUndefined();
    expect(parseBonePose({ geometries: [] })).toBeUndefined();
  });
});
