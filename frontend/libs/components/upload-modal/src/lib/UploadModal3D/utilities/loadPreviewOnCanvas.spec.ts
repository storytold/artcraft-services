import * as THREE from "three";
import { computeMeshBounds, loadPreviewOnCanvas } from "./loadPreviewOnCanvas";

const { renderers, pendingGltfLoads } = vi.hoisted(() => ({
  renderers: [] as Array<{
    dispose: ReturnType<typeof vi.fn>;
    forceContextLoss: ReturnType<typeof vi.fn>;
    setAnimationLoop: ReturnType<typeof vi.fn>;
  }>,
  pendingGltfLoads: [] as Array<(gltf: { scene: unknown }) => void>,
}));

// jsdom has no WebGL; only the lifecycle calls matter here.
vi.mock("three", async (importOriginal) => {
  const actual = await importOriginal<typeof import("three")>();
  class WebGLRenderer {
    dispose = vi.fn();
    forceContextLoss = vi.fn();
    setAnimationLoop = vi.fn();
    constructor() {
      renderers.push(this);
    }
    setSize() {}
    render() {}
  }
  return { ...actual, WebGLRenderer };
});

// Loads stay pending until a test resolves them, to model a load that
// finishes after the preview was superseded.
vi.mock("three/addons/loaders/GLTFLoader.js", () => ({
  GLTFLoader: class {
    load(_url: string, onLoad: (gltf: { scene: unknown }) => void) {
      pendingGltfLoads.push(onLoad);
    }
  },
}));

vi.mock("@sparkjsdev/spark", async () => {
  const { Object3D } = await import("three");
  return {
    SplatFileType: { SPZ: "spz" },
    SplatMesh: class extends Object3D {},
    SparkRenderer: class extends Object3D {},
  };
});

describe("loadPreviewOnCanvas dispose", () => {
  beforeEach(() => {
    renderers.length = 0;
    pendingGltfLoads.length = 0;
    vi.stubGlobal("URL", {
      ...URL,
      createObjectURL: vi.fn(() => "blob:preview"),
      revokeObjectURL: vi.fn(),
    });
  });

  it("stops the loop, frees the renderer and revokes the blob URL", () => {
    const canvas = attachedCanvas();
    const preview = loadPreviewOnCanvas({
      file: new File([], "model.glb"),
      canvas,
      statusCallback: vi.fn(),
    });

    preview.dispose();

    const renderer = renderers[0];
    expect(renderer.setAnimationLoop).toHaveBeenLastCalledWith(null);
    expect(renderer.dispose).toHaveBeenCalledTimes(1);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:preview");
  });

  it("keeps the context while the canvas is reused for the next file", () => {
    const preview = loadPreviewOnCanvas({
      file: new File([], "model.glb"),
      canvas: attachedCanvas(),
      statusCallback: vi.fn(),
    });

    preview.dispose();

    expect(renderers[0].forceContextLoss).not.toHaveBeenCalled();
  });

  it("releases the context once the canvas has left the DOM", () => {
    const canvas = attachedCanvas();
    const preview = loadPreviewOnCanvas({
      file: new File([], "model.glb"),
      canvas,
      statusCallback: vi.fn(),
    });

    canvas.remove();
    preview.dispose();

    expect(renderers[0].forceContextLoss).toHaveBeenCalledTimes(1);
  });

  it("ignores a model that finishes loading after dispose", () => {
    const statusCallback = vi.fn();
    const preview = loadPreviewOnCanvas({
      file: new File([], "model.glb"),
      canvas: attachedCanvas(),
      statusCallback,
    });

    preview.dispose();
    const lateModel = new THREE.Group();
    lateModel.add(new THREE.Mesh(new THREE.BoxGeometry()));
    pendingGltfLoads[0]({ scene: lateModel });

    expect(lateModel.children).toHaveLength(1);
    expect(statusCallback).not.toHaveBeenCalled();
  });
});

function attachedCanvas(): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.getContext = vi.fn(() => null) as never;
  document.body.appendChild(canvas);
  return canvas;
}

describe("computeMeshBounds", () => {
  it("frames transformed meshes and computes shared geometry bounds once", () => {
    const geometry = new THREE.BoxGeometry(1, 1, 1);
    const compute = vi.spyOn(geometry, "computeBoundingBox");
    const root = new THREE.Group();
    let parent: THREE.Object3D = root;
    for (let i = 0; i < 200; i++) {
      const mesh = new THREE.Mesh(geometry);
      mesh.position.x = 1;
      parent.add(mesh);
      parent = mesh;
    }

    const box = computeMeshBounds(root);

    expect(compute).toHaveBeenCalledTimes(1);
    expect(box.min.x).toBeCloseTo(0.5);
    expect(box.max.x).toBeCloseTo(200.5);
    expect(box.max.y).toBeCloseTo(0.5);
  });

  it("uses the current transform of a moved ancestor", () => {
    const parent = new THREE.Group();
    const child = new THREE.Group();
    child.add(new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1)));
    parent.add(child);
    parent.updateMatrixWorld(true);
    parent.position.x = 10;

    const box = computeMeshBounds(child);

    expect(box.min.x).toBeCloseTo(9.5);
    expect(box.max.x).toBeCloseTo(10.5);
  });
});
