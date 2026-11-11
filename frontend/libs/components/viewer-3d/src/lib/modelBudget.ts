// Decode budgets for untrusted 3D model files (scene assets, uploads,
// previews). The parsers we use (three's GLTFLoader, Spark's SPZ reader)
// allocate typed arrays sized by counts declared in the file before they have
// validated the rest of it, so a few bytes of metadata can request gigabytes
// of zero-filled memory. These checks read only the declared sizes and reject
// a file before any of that allocation happens.

/** Total decoded accessor storage a single glTF/GLB may request. */
export const MAX_GLTF_DECODED_BYTES = 1024 * 1024 * 1024; // 1 GiB

/** Largest splat count accepted from an SPZ header. */
export const MAX_SPZ_SPLATS = 10_000_000;

export class ModelBudgetError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ModelBudgetError";
  }
}

const GLB_MAGIC = 0x46546c67; // "glTF"
const GLB_CHUNK_JSON = 0x4e4f534a; // "JSON"

const COMPONENT_BYTES: Record<number, number> = {
  5120: 1, // BYTE
  5121: 1, // UNSIGNED_BYTE
  5122: 2, // SHORT
  5123: 2, // UNSIGNED_SHORT
  5125: 4, // UNSIGNED_INT
  5126: 4, // FLOAT
};

const TYPE_ITEM_SIZE: Record<string, number> = {
  SCALAR: 1,
  VEC2: 2,
  VEC3: 3,
  VEC4: 4,
  MAT2: 4,
  MAT3: 9,
  MAT4: 16,
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function readGltfJson(data: ArrayBuffer | string): any | undefined {
  try {
    if (typeof data === "string") return JSON.parse(data);
    if (data.byteLength >= 20) {
      const view = new DataView(data);
      if (view.getUint32(0, true) === GLB_MAGIC) {
        const chunkLength = view.getUint32(12, true);
        const chunkType = view.getUint32(16, true);
        if (chunkType !== GLB_CHUNK_JSON || 20 + chunkLength > data.byteLength) {
          return undefined;
        }
        const json = new TextDecoder().decode(new Uint8Array(data, 20, chunkLength));
        return JSON.parse(json);
      }
    }
    return JSON.parse(new TextDecoder().decode(new Uint8Array(data)));
  } catch {
    // Not parseable here; the loader reports its own error for malformed
    // files, and it cannot allocate from accessors it cannot read.
    return undefined;
  }
}

const isCount = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;

/**
 * Sum of the typed-array storage the glTF's accessors would allocate,
 * including sparse indices/values. Throws ModelBudgetError for invalid
 * counts. Returns 0 when the document cannot be read.
 */
export function gltfDecodedAccessorBytes(data: ArrayBuffer | string): number {
  const json = readGltfJson(data);
  const accessors: unknown = json?.accessors;
  if (!Array.isArray(accessors)) return 0;

  let total = 0;
  for (const accessor of accessors) {
    if (!accessor || typeof accessor !== "object") continue;
    const itemSize = TYPE_ITEM_SIZE[accessor.type as string];
    const componentBytes = COMPONENT_BYTES[accessor.componentType as number];
    if (!itemSize || !componentBytes) continue;
    if (!isCount(accessor.count)) {
      throw new ModelBudgetError("Model has an accessor with an invalid count.");
    }
    total += accessor.count * itemSize * componentBytes;

    const sparse = accessor.sparse;
    if (sparse && typeof sparse === "object") {
      if (!isCount(sparse.count)) {
        throw new ModelBudgetError("Model has a sparse accessor with an invalid count.");
      }
      const indexBytes = COMPONENT_BYTES[sparse.indices?.componentType as number] ?? 4;
      total += sparse.count * (indexBytes + itemSize * componentBytes);
    }
  }
  return total;
}

/** Throws ModelBudgetError when a glTF/GLB would decode past the budget. */
export function assertGltfWithinBudget(
  data: ArrayBuffer | string,
  maxBytes: number = MAX_GLTF_DECODED_BYTES,
): void {
  const total = gltfDecodedAccessorBytes(data);
  if (total > maxBytes) {
    throw new ModelBudgetError(
      `Model is too large to load (${Math.ceil(total / (1024 * 1024))} MB of geometry data).`,
    );
  }
}

type GltfParse = (
  data: ArrayBuffer | string,
  path: string,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  onLoad: (gltf: any) => void,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  onError?: (error: any) => void,
) => void;

/**
 * Enforces the decode budget on every parse a GLTFLoader instance performs,
 * including those `load()` makes after fetching. Returns the same loader.
 */
export function applyGltfBudget<T extends { parse: GltfParse }>(
  loader: T,
  maxBytes: number = MAX_GLTF_DECODED_BYTES,
): T {
  const parse = loader.parse.bind(loader) as GltfParse;
  loader.parse = ((data, path, onLoad, onError) => {
    try {
      assertGltfWithinBudget(data, maxBytes);
    } catch (error) {
      if (onError) onError(error);
      else throw error;
      return;
    }
    parse(data, path, onLoad, onError);
  }) as T["parse"];
  return loader;
}

const SPZ_MAGIC = 0x5053474e; // "NGSP"
const SPZ_HEADER_BYTES = 16;

// Gunzips just enough of the stream to read the SPZ header, never more than
// a few chunks, and stops.
async function readGzipPrefix(bytes: Uint8Array, length: number): Promise<Uint8Array | undefined> {
  if (typeof DecompressionStream === "undefined" || typeof ReadableStream === "undefined") {
    return undefined;
  }
  // Feed the compressed bytes in small slices on demand, so only as much is
  // inflated as the header needs.
  let offset = 0;
  const source = new ReadableStream<Uint8Array>({
    pull(controller) {
      if (offset >= bytes.length) {
        controller.close();
        return;
      }
      controller.enqueue(bytes.slice(offset, offset + 16 * 1024));
      offset += 16 * 1024;
    },
  });
  const reader = source.pipeThrough(new DecompressionStream("gzip")).getReader();
  const out = new Uint8Array(length);
  let filled = 0;
  try {
    while (filled < length) {
      const { value, done } = await reader.read();
      if (done || !value) break;
      const take = Math.min(value.length, length - filled);
      out.set(value.subarray(0, take), filled);
      filled += take;
    }
  } catch {
    return undefined;
  } finally {
    reader.cancel().catch(() => undefined);
  }
  return filled === length ? out : undefined;
}

/**
 * Reads the SPZ header and throws ModelBudgetError when it declares more
 * splats than MAX_SPZ_SPLATS. The SPZ decoder sizes its storage from this
 * count and reads exactly that much data, so bounding it bounds the decode.
 * Non-gzip data and unreadable headers are left for the decoder to reject.
 */
export async function assertSpzWithinBudget(
  data: ArrayBuffer,
  maxSplats: number = MAX_SPZ_SPLATS,
): Promise<void> {
  const bytes = new Uint8Array(data);
  if (bytes.length < 2 || bytes[0] !== 0x1f || bytes[1] !== 0x8b) return;
  const header = await readGzipPrefix(bytes, SPZ_HEADER_BYTES);
  if (!header) return;
  const view = new DataView(header.buffer, header.byteOffset, header.byteLength);
  if (view.getUint32(0, true) !== SPZ_MAGIC) return;
  const numPoints = view.getUint32(8, true);
  if (numPoints > maxSplats) {
    throw new ModelBudgetError(
      `Splat is too large to load (${numPoints.toLocaleString("en-US")} splats).`,
    );
  }
}
