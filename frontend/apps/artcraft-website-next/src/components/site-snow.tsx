import type { CSSProperties } from "react";
import styles from "./site-snow.module.css";

// Fixed offsets keep the pattern consistent across server renders.
const flakes = Array.from({ length: 36 }, (_, index) => ({
  left: `${(index * 37.7 + 3) % 100}%`,
  "--size": `${((2 + (index * 13 % 23) / 10) * 1.2).toFixed(2)}px`,
  "--drift": `${(index * 23 % 73) - 36}px`,
  "--duration": `${15 + index * 7 % 13}s`,
  "--delay": `-${(index * 7.3) % 27}s`,
  opacity: 0.32 + (index * 11 % 31) / 100,
}) satisfies CSSProperties & Record<`--${string}`, string>);

export default function SiteSnow() {
  return (
    <div className={styles.snow} aria-hidden="true" data-site-snow>
      {flakes.map((style, index) => (
        <span key={index} className={styles.flake} style={style} />
      ))}
    </div>
  );
}
