import { Matrix4, Object3D, Quaternion, Vector3 } from "three";

// Saved poses are the JSON of the hip bone subtree (Object3D.toJSON). Only the
// per-bone local transforms and the child order are read back; everything
// else in the document (geometries, materials, images, ...) is ignored, so a
// scene file cannot make the pose loader build arbitrary three.js objects.
// Real rigs have well under a hundred bones.
export const MAX_POSE_BONES = 1024;
export const MAX_POSE_DEPTH = 128;

export interface BonePoseNode {
  position: Vector3;
  quaternion: Quaternion;
  scale: Vector3;
  children: BonePoseNode[];
}

const isFiniteNumberArray = (value: unknown, length: number): value is number[] =>
  Array.isArray(value) &&
  value.length === length &&
  value.every((n) => typeof n === "number" && Number.isFinite(n));

// Reads a node's local transform from either its packed matrix or its
// separate position/quaternion/scale fields, defaulting to identity.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function readTransform(data: any): Pick<BonePoseNode, "position" | "quaternion" | "scale"> {
  const position = new Vector3();
  const quaternion = new Quaternion();
  const scale = new Vector3(1, 1, 1);
  if (isFiniteNumberArray(data.matrix, 16)) {
    new Matrix4().fromArray(data.matrix).decompose(position, quaternion, scale);
    return { position, quaternion, scale };
  }
  if (isFiniteNumberArray(data.position, 3)) position.fromArray(data.position);
  if (isFiniteNumberArray(data.quaternion, 4)) quaternion.fromArray(data.quaternion);
  if (isFiniteNumberArray(data.scale, 3)) scale.fromArray(data.scale);
  return { position, quaternion, scale };
}

// Reads the bone-transform subset of an Object3D.toJSON() document into a
// plain tree. Returns undefined when the document has no usable root; nodes
// past the bone or depth budget are dropped.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function parseBonePose(boneJSON: any): BonePoseNode | undefined {
  let remaining = MAX_POSE_BONES;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const readNode = (data: any, depth: number): BonePoseNode | undefined => {
    if (!data || typeof data !== "object" || depth > MAX_POSE_DEPTH || remaining <= 0) {
      return undefined;
    }
    remaining--;

    const { position, quaternion, scale } = readTransform(data);
    const children: BonePoseNode[] = [];
    if (Array.isArray(data.children)) {
      for (const child of data.children) {
        if (remaining <= 0) break;
        const node = readNode(child, depth + 1);
        if (node) children.push(node);
      }
    }
    return { position, quaternion, scale, children };
  };

  return readNode(boneJSON?.object, 0);
}

export class BoneJSONHelper {

  model: Object3D;

  constructor(model: Object3D) {
    this.model = model;
  }

  private findHipBone(objectGroup: Object3D[]): Object3D | undefined {
    let hipBone = undefined;
    for (const child of objectGroup) {
      if (child.type === "Bone" && child.name.toLowerCase().includes("mixamorighips")) {
        hipBone = child;
        break;
      }
    }

    return hipBone;
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  public toJSON(): any {
    if (!this.model.userData.isCharacter) {
      return;
    }

    if (this.model.children.length < 1) {
      return undefined;
    }

    // Find the hip bone
    const hipBone = this.findHipBone(this.model.children[0].children);

    if (!hipBone) {
      console.error("No hip bone found in the model.");
      return undefined;
    }

    return hipBone.toJSON();
  }

  /* Applies the bone tree in a recursive BFS manner */
  private applyBoneTree(hipBone: Object3D, boneTree: BonePoseNode): void {
    hipBone.position.copy(boneTree.position);
    hipBone.quaternion.copy(boneTree.quaternion);
    hipBone.scale.copy(boneTree.scale);

    const hipChildren = hipBone.children;
    const treeChildren = boneTree.children;

    // NOTE: Assumption made that the lengths match!
    // This shouldn't be a problem unless the skeletal model is changed
    for (let i = 0; i < hipChildren.length; i++) {
      const hipChild = hipChildren[i];
      const treeChild = treeChildren[i];
      if (hipChild && treeChild) {
        this.applyBoneTree(hipChild, treeChild);
      }
    }
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  public poseFromBoneJSON(boneJSON: any): void {
    const hipBone = this.findHipBone(this.model.children[0].children);
    if (!hipBone) {
      return;
    }

    const boneTree = parseBonePose(boneJSON);
    if (!boneTree) {
      return;
    }
    this.applyBoneTree(hipBone, boneTree);
  }

}
