import { motion, useReducedMotion } from "framer-motion";

export function SidebarActiveIndicator() {
  const reduceMotion = useReducedMotion();

  return (
    <motion.span
      aria-hidden="true"
      layoutId="sidebar-active-indicator"
      className="pointer-events-none absolute inset-y-1.5 left-0 z-10 w-0.5 bg-white"
      transition={
        reduceMotion
          ? { duration: 0 }
          : { type: "spring", stiffness: 520, damping: 40, mass: 0.7 }
      }
    />
  );
}
