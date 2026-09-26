"use client";

import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { MeshoptDecoder } from "three/examples/jsm/libs/meshopt_decoder.module.js";
import { ArrowLeftIcon, ArrowRightIcon, LoaderCircleIcon, MinusIcon, PlusIcon, RotateCcwIcon } from "lucide-react";
import { Button } from "@/components/ui";
import type { SplatMesh } from "@sparkjsdev/spark";

export default function MediaViewer3D({ url, kind, format }: {
  url: string; kind: "mesh" | "splat"; format: string;
}) {
  const container = useRef<HTMLDivElement>(null);
  const resetView = useRef(() => {});
  const frameView = useRef(() => {});
  const zoomView = useRef((_factor: number) => {});
  const rotateView = useRef((_angle: number) => {});
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [error, setError] = useState("");

  useEffect(() => {
    const element = container.current;
    if (!element) return;
    let cancelled = false;
    let model: THREE.Object3D | null = null;
    let splat: SplatMesh | null = null;
    let splatDisposed = false;
    let mixer: THREE.AnimationMixer | null = null;
    let renderer: THREE.WebGLRenderer;
    setStatus("loading");

    try {
      renderer = new THREE.WebGLRenderer({ antialias: kind === "mesh", alpha: true });
    } catch {
      setError("Interactive 3D needs WebGL. You can still download the file or open it in ArtCraft.");
      setStatus("error");
      return;
    }

    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor(0x000000, 0);
    renderer.domElement.setAttribute("aria-label", kind === "splat" ? "Interactive 3D splat" : "Interactive 3D mesh");
    renderer.domElement.tabIndex = 0;
    element.appendChild(renderer.domElement);
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(45, 1, 0.01, 10000);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.listenToKeyEvents(renderer.domElement);
    scene.add(new THREE.HemisphereLight(0xffffff, 0x6b7280, 2));
    const key = new THREE.DirectionalLight(0xffffff, 3);
    key.position.set(4, 8, 6);
    scene.add(key);
    const fill = new THREE.DirectionalLight(0xd6e7ff, 1.5);
    fill.position.set(-4, 2, -3);
    scene.add(fill);
    scene.add(new THREE.AmbientLight(0xffffff, 1.5));

    function disposeSplat() {
      if (!splat || splatDisposed) return;
      splatDisposed = true;
      splat.dispose();
    }

    function resize() {
      const width = Math.max(element!.clientWidth, 1);
      const height = Math.max(element!.clientHeight, 1);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
      renderer.setSize(width, height);
    }
    const observer = new ResizeObserver(resize);
    observer.observe(element);
    resize();

    function failed(message: string) {
      if (cancelled) return;
      renderer.setAnimationLoop(null);
      setError(message);
      setStatus("error");
    }
    function lost(event: Event) {
      event.preventDefault();
      failed("The 3D viewer lost its graphics connection. Reload this page to try again.");
    }
    renderer.domElement.addEventListener("webglcontextlost", lost);

    async function load() {
      try {
        let bounds: THREE.Box3;
        if (kind === "splat") {
          const { SplatMesh, SplatFileType } = await import("@sparkjsdev/spark");
          if (cancelled) return;
          const fileType = Object.values(SplatFileType).find((value) => value === format);
          const loaded = new SplatMesh({ url, fileType });
          // Keep ownership across await so failed decodes also release resources.
          splat = loaded;
          await loaded.initialized;
          if (cancelled) { disposeSplat(); return; }
          // Match the coordinate convention used by the Vite viewer.
          loaded.rotation.z = Math.PI;
          loaded.updateMatrixWorld(true);
          bounds = loaded.getBoundingBox().applyMatrix4(loaded.matrixWorld);
          model = loaded;
        } else {
          const loaded = await loadMesh(url, format);
          if (cancelled) { disposeModel(loaded.model); return; }
          model = loaded.model;
          bounds = new THREE.Box3().setFromObject(model);
          if (bounds.isEmpty()) {
            // Skeleton-only GLBs still need a visible rig and finite framing.
            model.updateMatrixWorld(true);
            model.traverse((node) => bounds.expandByPoint(node.getWorldPosition(new THREE.Vector3())));
            scene.add(new THREE.SkeletonHelper(model));
          }
          if (loaded.animations.length) {
            mixer = new THREE.AnimationMixer(model);
            mixer.clipAction(loaded.animations[0]).play();
          }
        }
        scene.add(model);
        const center = bounds.getCenter(new THREE.Vector3());
        const size = bounds.getSize(new THREE.Vector3());
        const radius = Math.max(size.length() / 2, 0.5);
        frameView.current = () => {
          const halfFov = THREE.MathUtils.degToRad(camera.fov / 2);
          const distance = radius / Math.sin(Math.atan(Math.tan(halfFov) * Math.min(camera.aspect, 1))) * 1.15;
          camera.near = Math.max(radius / 1000, 0.001);
          camera.far = Math.max(radius * 100, 1000);
          camera.position.copy(center).add(new THREE.Vector3(1, 0.5, 1).normalize().multiplyScalar(distance));
          camera.updateProjectionMatrix();
          controls.target.copy(center);
          controls.maxDistance = radius * 50;
          controls.update();
        };
        resetView.current = () => {
          frameView.current();
          if (kind === "splat") {
            // World captures are meant to be viewed from their capture origin.
            // Framing their outer bounds shows the back of the environment.
            // Preserve the Vite viewer's initial orbit near that origin; the
            // separate Frame all action still works for object-sized splats.
            camera.near = 0.01;
            camera.position.set(0.85, 0, 0.85);
            camera.updateProjectionMatrix();
            controls.target.set(0, 0, 0);
            controls.update();
          }
        };
        resetView.current();
        zoomView.current = (factor) => {
          const offset = camera.position.clone().sub(controls.target);
          offset.multiplyScalar(factor).clampLength(camera.near * 2, controls.maxDistance);
          camera.position.copy(controls.target).add(offset);
          controls.update();
        };
        rotateView.current = (angle) => {
          const offset = camera.position.clone().sub(controls.target).applyAxisAngle(camera.up, angle);
          camera.position.copy(controls.target).add(offset);
          controls.update();
        };
        let previous = performance.now();
        renderer.setAnimationLoop(() => {
          try {
            const now = performance.now();
            mixer?.update(Math.min((now - previous) / 1000, 0.1));
            previous = now;
            controls.update();
            renderer.render(scene, camera);
          } catch {
            failed("This file could not be rendered by your browser. The original is available to download.");
          }
        });
        setStatus("ready");
      } catch {
        disposeSplat();
        failed("This 3D file could not be loaded. Download the original or open it in ArtCraft.");
      }
    }
    void load();

    return () => {
      cancelled = true;
      observer.disconnect();
      renderer.setAnimationLoop(null);
      renderer.domElement.removeEventListener("webglcontextlost", lost);
      controls.dispose();
      mixer?.stopAllAction();
      if (model && mixer) mixer.uncacheRoot(model);
      // An in-flight splat is disposed after its decode finishes above.
      if (splat?.isInitialized) disposeSplat();
      disposeModel(scene);
      renderer.dispose();
      renderer.forceContextLoss();
      renderer.domElement.remove();
    };
  }, [url, kind, format]);

  return (
    <div className="relative h-[60svh] min-h-[360px] w-full lg:h-[700px]" data-lenis-prevent>
      <div ref={container} className="absolute inset-0 touch-none" />
      {status === "loading" && (
        <div role="status" className="absolute inset-0 flex items-center justify-center gap-3 bg-bg-sunken/90 text-muted">
          <LoaderCircleIcon aria-hidden className="h-5 w-5 animate-spin motion-reduce:animate-none" />
          <span className="hud-label">Loading 3D {kind === "splat" ? "splat" : "mesh"}</span>
        </div>
      )}
      {status === "error" && <p role="alert" className="absolute inset-0 flex items-center justify-center bg-bg-sunken p-10 text-center text-muted">{error}</p>}
      {status === "ready" && (
        <div className="absolute inset-x-0 bottom-0 flex flex-wrap items-center justify-between gap-3 border-t border-line bg-bg/90 p-4 backdrop-blur-md">
          <p className="hud-label text-muted">Drag to orbit · Scroll / pinch to zoom · Right-drag to pan</p>
          <div className="flex flex-wrap gap-2" role="group" aria-label="3D view controls">
            <Button variant="secondary" size="sm" aria-label="Rotate left" onClick={() => rotateView.current(-Math.PI / 8)}><ArrowLeftIcon aria-hidden className="h-3 w-3" /></Button>
            <Button variant="secondary" size="sm" aria-label="Rotate right" onClick={() => rotateView.current(Math.PI / 8)}><ArrowRightIcon aria-hidden className="h-3 w-3" /></Button>
            <Button variant="secondary" size="sm" aria-label="Zoom in" onClick={() => zoomView.current(0.8)}><PlusIcon aria-hidden className="h-3 w-3" /></Button>
            <Button variant="secondary" size="sm" aria-label="Zoom out" onClick={() => zoomView.current(1.25)}><MinusIcon aria-hidden className="h-3 w-3" /></Button>
            {kind === "splat" && <Button variant="secondary" size="sm" onClick={() => frameView.current()}>Frame all</Button>}
            <Button variant="secondary" size="sm" onClick={() => resetView.current()}><RotateCcwIcon aria-hidden className="h-3 w-3" />Reset view</Button>
          </div>
        </div>
      )}
    </div>
  );
}

async function loadMesh(url: string, format: string): Promise<{ model: THREE.Object3D; animations: THREE.AnimationClip[] }> {
  if (format === "fbx") {
    const { FBXLoader } = await import("three/examples/jsm/loaders/FBXLoader.js");
    const model = await new FBXLoader().loadAsync(url);
    return { model, animations: model.animations };
  }
  if (format === "obj") {
    const { OBJLoader } = await import("three/examples/jsm/loaders/OBJLoader.js");
    return { model: await new OBJLoader().loadAsync(url), animations: [] };
  }
  if (format === "ply" || format === "stl") {
    const geometry = format === "ply"
      ? await new (await import("three/examples/jsm/loaders/PLYLoader.js")).PLYLoader().loadAsync(url)
      : await new (await import("three/examples/jsm/loaders/STLLoader.js")).STLLoader().loadAsync(url);
    geometry.computeVertexNormals();
    return { model: new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({ color: 0xb9c5d4, vertexColors: !!geometry.getAttribute("color"), side: THREE.DoubleSide })), animations: [] };
  }
  if (!["glb", "gltf", ""].includes(format)) throw new Error("Unsupported mesh format");
  const loader = new GLTFLoader().setMeshoptDecoder(MeshoptDecoder);
  const gltf = await loader.loadAsync(url);
  return { model: gltf.scene, animations: gltf.animations };
}

function disposeModel(model: THREE.Object3D) {
  const textures = new Set<THREE.Texture>();
  model.traverse((node) => {
    const mesh = node as THREE.Mesh;
    mesh.geometry?.dispose();
    const materials = Array.isArray(mesh.material) ? mesh.material : mesh.material ? [mesh.material] : [];
    for (const material of materials) {
      for (const value of Object.values(material)) if (value instanceof THREE.Texture) textures.add(value);
      material.dispose();
    }
  });
  textures.forEach((texture) => texture.dispose());
}
