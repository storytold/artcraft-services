import { deflateSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { FbxBudgetError, MAX_FBX_ARRAY_BYTES, assertFbxWithinBudget } from "./fbxBudget";

describe("assertFbxWithinBudget", () => {
  it("accepts a compressed array that matches its declared size", async () => {
    const values = new Float32Array([1, 2, 3, 4]);
    const fbx = makeFbx(values.length, deflateSync(new Uint8Array(values.buffer)));
    await expect(assertFbxWithinBudget(fbx)).resolves.toBeUndefined();
  });

  it("rejects a compressed array that inflates past its declared size", async () => {
    const bomb = deflateSync(new Uint8Array(16 * 1024 * 1024));
    expect(bomb.length).toBeLessThan(64 * 1024);
    const fbx = makeFbx(4, bomb);
    await expect(assertFbxWithinBudget(fbx)).rejects.toBeInstanceOf(FbxBudgetError);
  });

  it("rejects arrays declaring more than the total budget", async () => {
    const fbx = makeFbx(MAX_FBX_ARRAY_BYTES / 4 + 1, deflateSync(new Uint8Array(4)));
    await expect(assertFbxWithinBudget(fbx)).rejects.toBeInstanceOf(FbxBudgetError);
  });

  it("ignores non-binary files", async () => {
    const ascii = new TextEncoder().encode("; FBX 7.4.0 project file\n").buffer;
    await expect(assertFbxWithinBudget(ascii)).resolves.toBeUndefined();
  });
});

// Binary FBX 7400 with one top-level node "Vertices" holding a compressed
// float array, a NULL record, and a zeroed footer.
function makeFbx(arrayLength: number, compressed: Uint8Array): ArrayBuffer {
  const magic = new TextEncoder().encode("Kaydara FBX Binary  \0");
  const name = new TextEncoder().encode("Vertices");
  const headerBytes = 27;
  const propBytes = 1 + 12 + compressed.length;
  const nodeBytes = 13 + name.length + propBytes;
  const total = headerBytes + nodeBytes + 13 + 176;
  const out = new Uint8Array(total);
  const view = new DataView(out.buffer);
  out.set(magic, 0);
  out[21] = 0x1a;
  view.setUint32(23, 7400, true);
  let o = headerBytes;
  view.setUint32(o, headerBytes + nodeBytes, true);
  view.setUint32(o + 4, 1, true);
  view.setUint32(o + 8, propBytes, true);
  view.setUint8(o + 12, name.length);
  out.set(name, o + 13);
  o += 13 + name.length;
  out[o] = "f".charCodeAt(0);
  view.setUint32(o + 1, arrayLength, true);
  view.setUint32(o + 5, 1, true);
  view.setUint32(o + 9, compressed.length, true);
  out.set(compressed, o + 13);
  return out.buffer;
}
