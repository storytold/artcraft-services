import { memo } from "react";
import { Shape } from "react-konva";
import Konva from "konva";
import { Rect } from "./types";
import { effectiveGridSpacing, gridDotLayout } from "./layout/gridDots";

interface Props {
  // Visible region in stage (document) coordinates. Driven by pan/zoom.
  viewport: Rect;
  spacing: number;
  zoom: number;
  dotColor?: string;
  // On-screen radius in CSS pixels. Converted to stage units so dots stay
  // the same visual size regardless of zoom.
  dotRadiusPx?: number;
}

// Vector dotted grid that follows the visible viewport. Dots stay anchored
// at fixed stage coords (multiples of the effective spacing); zooming out
// coarsens the grid in powers of 2 so we never draw tens of thousands of
// arcs per frame.
const MoodboardBackgroundInner = ({
  viewport,
  spacing,
  zoom,
  dotColor = "rgba(255,255,255,0.18)",
  dotRadiusPx = 1.2,
}: Props) => {
  const effectiveSpacing = effectiveGridSpacing(spacing, zoom);
  const radius = dotRadiusPx / Math.max(zoom, 1e-6);

  return (
    <Shape
      x={0}
      y={0}
      listening={false}
      sceneFunc={(ctx: Konva.Context, shape) => {
        if (effectiveSpacing === null) return;
        const grid = gridDotLayout(viewport, effectiveSpacing);
        if (grid === null) return;
        ctx.fillStyle = dotColor;
        ctx.beginPath();
        for (let i = 0; i < grid.cols; i++) {
          const x = grid.startX + i * effectiveSpacing;
          for (let j = 0; j < grid.rows; j++) {
            const y = grid.startY + j * effectiveSpacing;
            ctx.moveTo(x + radius, y);
            ctx.arc(x, y, radius, 0, Math.PI * 2);
          }
        }
        ctx.fill();
        ctx.fillStrokeShape(shape);
      }}
    />
  );
};

export const MoodboardBackground = memo(MoodboardBackgroundInner);
