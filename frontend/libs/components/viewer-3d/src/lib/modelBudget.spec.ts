import { gzipSync } from "node:zlib";
import { describe, expect, it, vi } from "vitest";
import {
  MAX_GLTF_DECODED_BYTES,
  ModelBudgetError,
  applyGltfBudget,
  assertGltfWithinBudget,
  assertSpzWithinBudget,
  gltfDecodedAccessorBytes,
} from "./modelBudget";

describe("glTF decode budget", () => {
  it("sums accessor and sparse storage", () => {
    const gltf = JSON.stringify({
      accessors: [
        { count: 10, type: "VEC3", componentType: 5126 },
        { count: 4, type: "SCALAR", componentType: 5123, sparse: { count: 2, indices: { componentType: 5121 } } },
      ],
    });
    expect(gltfDecodedAccessorBytes(gltf)).toBe(10 * 3 * 4 + 4 * 2 + 2 * (1 + 2));
  });

  it("rejects a compact GLB whose accessors request too much memory", () => {
    const glb = makeGlb({ accessors: [{ count: 2 ** 30, type: "MAT4", componentType: 5126 }] });
    expect(glb.byteLength).toBeLessThan(200);
    expect(() => assertGltfWithinBudget(glb)).toThrow(ModelBudgetError);
  });

  it("rejects invalid counts and accepts normal models", () => {
    expect(() => assertGltfWithinBudget(JSON.stringify({ accessors: [{ count: -1, type: "VEC3", componentType: 5126 }] }))).toThrow(ModelBudgetError);
    expect(() => assertGltfWithinBudget(makeGlb({ accessors: [{ count: 1000, type: "VEC3", componentType: 5126 }] }))).not.toThrow();
    expect(MAX_GLTF_DECODED_BYTES).toBeGreaterThan(0);
  });

  it("applyGltfBudget routes over-budget parses to onError without parsing", () => {
    const parse = vi.fn();
    const loader = applyGltfBudget({ parse });
    const onError = vi.fn();
    loader.parse(makeGlb({ accessors: [{ count: 2 ** 31, type: "VEC4", componentType: 5126 }] }), "", vi.fn(), onError);
    expect(parse).not.toHaveBeenCalled();
    expect(onError.mock.calls[0][0]).toBeInstanceOf(ModelBudgetError);

    loader.parse(makeGlb({ accessors: [] }), "", vi.fn(), onError);
    expect(parse).toHaveBeenCalledTimes(1);
  });
});

describe("SPZ decode budget", () => {
  it("rejects a header declaring too many splats", async () => {
    await expect(assertSpzWithinBudget(makeSpz(4_000_000_000))).rejects.toBeInstanceOf(ModelBudgetError);
  });

  it("accepts a normal header and ignores non-SPZ data", async () => {
    await expect(assertSpzWithinBudget(makeSpz(100_000))).resolves.toBeUndefined();
    await expect(assertSpzWithinBudget(new Uint8Array([1, 2, 3]).buffer)).resolves.toBeUndefined();
  });
});

function makeGlb(json: unknown): ArrayBuffer {
  let text = JSON.stringify(json);
  while (text.length % 4) text += " ";
  const body = new TextEncoder().encode(text);
  const out = new ArrayBuffer(20 + body.length);
  const view = new DataView(out);
  view.setUint32(0, 0x46546c67, true);
  view.setUint32(4, 2, true);
  view.setUint32(8, out.byteLength, true);
  view.setUint32(12, body.length, true);
  view.setUint32(16, 0x4e4f534a, true);
  new Uint8Array(out, 20).set(body);
  return out;
}

function makeSpz(numPoints: number): ArrayBuffer {
  const header = new DataView(new ArrayBuffer(16));
  header.setUint32(0, 0x5053474e, true);
  header.setUint32(4, 2, true);
  header.setUint32(8, numPoints, true);
  const gz = gzipSync(new Uint8Array(header.buffer));
  return gz.buffer.slice(gz.byteOffset, gz.byteOffset + gz.byteLength);
}
