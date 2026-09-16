import { type ReactNode } from "react";
import { twMerge } from "tailwind-merge";
import { FrameCornerMarks } from "../ui/frame-corner-marks";

/** A contained section frame shared by the auth pages. */
export function AuthPageFrame({ children, wide = false }: {
  children: ReactNode;
  wide?: boolean;
}) {
  return (
    <div className="relative isolate flex min-h-dvh items-center justify-center overflow-hidden bg-ui-background p-4 text-white sm:py-10">
      <div className={twMerge("relative w-full", wide ? "max-w-lg lg:max-w-5xl" : "max-w-lg")}>
        <div className="relative border border-white/20 bg-ui-controls">
          <div className={twMerge("flex", wide ? "lg:min-h-[560px]" : "flex-col")}>
            {children}
          </div>
        </div>
        <FrameCornerMarks />
      </div>
    </div>
  );
}
