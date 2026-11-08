// Decompression budget for binary FBX uploads. three's FBXLoader inflates
// every compressed array property with fflate's unzlibSync, which has no
// output limit, so a small, highly compressible file can expand to far more
// memory than its size suggests. This preflight walks the binary node tree
// the same way FBXLoader does and, before any parse, checks:
//   - declared array sizes against a per-file total, and
//   - each compressed array by inflating it in a stream that is cancelled as
//     soon as it produces more bytes than the array declares.
// ASCII FBX files have no compressed arrays and pass through unchanged.

/** Total decoded array storage a single FBX file may declare. */
export const MAX_FBX_ARRAY_BYTES = 512 * 1024 * 1024; // 512 MiB
const MAX_FBX_NODE_DEPTH = 256;

const FBX_BINARY_MAGIC = "Kaydara FBX Binary  \0";
const INFLATE_INPUT_SLICE = 16 * 1024;

const ARRAY_ELEMENT_BYTES: Record<string, number> = {
  b: 1,
  c: 1,
  d: 8,
  f: 4,
  i: 4,
  l: 8,
};

export class FbxBudgetError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FbxBudgetError";
  }
}

/**
 * Throws FbxBudgetError when a binary FBX declares or inflates more array
 * data than the budget allows, or a compressed array inflates past its
 * declared size. Structural problems are left for FBXLoader to report.
 */
export async function assertFbxWithinBudget(
  buffer: ArrayBuffer,
  maxBytes: number = MAX_FBX_ARRAY_BYTES,
): Promise<void> {
  const bytes = new Uint8Array(buffer);
  if (!hasBinaryMagic(bytes)) return;
  const view = new DataView(buffer);
  if (bytes.length < 27) return;
  const version = view.getUint32(23, true);
  const wide = version >= 7500;

  let offset = 27;
  let declared = 0;
  const compressed: { start: number; length: number; expected: number }[] = [];

  const readOffset = (): number => {
    if (wide) {
      const value = Number(view.getBigUint64(offset, true));
      offset += 8;
      return value;
    }
    const value = view.getUint32(offset, true);
    offset += 4;
    return value;
  };

  // Mirrors FBXLoader's BinaryParser.parseNode; returns false on a NULL
  // record. Throws RangeError on truncated data (caught below).
  const walkNode = (depth: number): boolean => {
    if (depth > MAX_FBX_NODE_DEPTH) {
      throw new FbxBudgetError("FBX file nests nodes too deeply.");
    }
    const endOffset = readOffset();
    const numProperties = readOffset();
    readOffset(); // property list length
    const nameLength = view.getUint8(offset);
    offset += 1 + nameLength;
    if (endOffset === 0) return false;

    for (let i = 0; i < numProperties; i++) {
      walkProperty();
    }
    while (endOffset > offset) {
      const before = offset;
      walkNode(depth + 1);
      if (offset <= before) break;
    }
    return true;
  };

  const walkProperty = (): void => {
    const type = String.fromCharCode(view.getUint8(offset));
    offset += 1;
    switch (type) {
      case "C":
        offset += 1;
        return;
      case "Y":
        offset += 2;
        return;
      case "F":
      case "I":
        offset += 4;
        return;
      case "D":
      case "L":
        offset += 8;
        return;
      case "R":
      case "S": {
        const length = view.getUint32(offset, true);
        offset += 4 + length;
        return;
      }
      default: {
        const elementBytes = ARRAY_ELEMENT_BYTES[type];
        if (!elementBytes) {
          throw new RangeError(`Unknown FBX property type ${type}`);
        }
        const arrayLength = view.getUint32(offset, true);
        const encoding = view.getUint32(offset + 4, true);
        const compressedLength = view.getUint32(offset + 8, true);
        offset += 12;
        const expected = arrayLength * elementBytes;
        declared += expected;
        if (declared > maxBytes) {
          throw new FbxBudgetError("FBX file is too large to convert.");
        }
        if (encoding === 0) {
          offset += expected;
        } else if (encoding === 1) {
          compressed.push({ start: offset, length: compressedLength, expected });
          offset += compressedLength;
        } else {
          throw new RangeError(`Unknown FBX array encoding ${encoding}`);
        }
      }
    }
  };

  try {
    while (offset + 176 < bytes.length) {
      if (!walkNode(0) && offset + 176 >= bytes.length) break;
    }
  } catch (error) {
    if (error instanceof FbxBudgetError) throw error;
    // Truncated or unusual layout: stop walking; FBXLoader reports its own
    // error for malformed files. Arrays already found are still checked.
  }

  for (const array of compressed) {
    const end = Math.min(array.start + array.length, bytes.length);
    await inflateWithLimit(bytes.subarray(array.start, end), array.expected);
  }
}

function hasBinaryMagic(bytes: Uint8Array): boolean {
  if (bytes.length < FBX_BINARY_MAGIC.length) return false;
  for (let i = 0; i < FBX_BINARY_MAGIC.length; i++) {
    if (bytes[i] !== FBX_BINARY_MAGIC.charCodeAt(i)) return false;
  }
  return true;
}

// Inflates a zlib stream, discarding output, and throws as soon as it
// produces more than `limit` bytes. Input is fed in small slices on demand,
// so each step expands a bounded amount.
async function inflateWithLimit(input: Uint8Array, limit: number): Promise<void> {
  if (typeof DecompressionStream === "undefined" || typeof ReadableStream === "undefined") {
    return;
  }
  let offset = 0;
  const source = new ReadableStream<Uint8Array>({
    pull(controller) {
      if (offset >= input.length) {
        controller.close();
        return;
      }
      controller.enqueue(input.slice(offset, offset + INFLATE_INPUT_SLICE));
      offset += INFLATE_INPUT_SLICE;
    },
  });
  const reader = source.pipeThrough(new DecompressionStream("deflate")).getReader();
  let produced = 0;
  try {
    for (;;) {
      let chunk: ReadableStreamReadResult<Uint8Array>;
      try {
        chunk = await reader.read();
      } catch {
        // FBXLoader's fflate inflate does not verify the Adler-32 trailer.
        throw new FbxBudgetError("FBX file contains a compressed array that fails to inflate.");
      }
      if (chunk.done) return;
      produced += chunk.value.byteLength;
      if (produced > limit) {
        throw new FbxBudgetError("FBX file contains an array that expands past its declared size.");
      }
    }
  } finally {
    reader.cancel().catch(() => undefined);
  }
}
