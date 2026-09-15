"use client";

import { useEffect } from "react";
import { defineTunables, useTunerStore } from "@/lib/tuner";

// Live control over the content stage's fluid width (see --stage-max in
// globals.css). Writes the root variable only when the knobs change — CSS
// does the actual responsive work.
const stageTuner = defineTunables("stage", "Stage", {
  stageMax: {
    label: "Stage max px",
    min: 1280,
    max: 1920,
    step: 10,
    default: 1600,
    info: "Ceiling of the content stage's fluid width — reached on wide monitors.",
  },
  stageVw: {
    label: "Stage vw %",
    min: 50,
    max: 100,
    step: 1,
    default: 72,
    info: "How much of the viewport the stage tracks between the 1280px floor and the ceiling.",
  },
});

export default function StageWidth() {
  useEffect(() => {
    const apply = () => {
      const t = stageTuner.read();
      document.documentElement.style.setProperty(
        "--stage-max",
        `clamp(1280px, ${t.stageVw}vw, ${t.stageMax}px)`,
      );
    };
    apply();
    return useTunerStore.subscribe(apply);
  }, []);
  return null;
}
