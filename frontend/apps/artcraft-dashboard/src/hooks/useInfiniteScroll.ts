import { useEffect, useRef } from "react";

/**
 * Calls `onLoadMore` when the element the returned ref is attached to (at the end of a list)
 * scrolls into view.
 */
export function useInfiniteScroll<T extends Element>(onLoadMore: () => void) {
  const sentinelRef = useRef<T | null>(null);

  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) onLoadMore();
      },
      { threshold: 0.1 },
    );

    if (sentinelRef.current) {
      observer.observe(sentinelRef.current);
    }

    return () => observer.disconnect();
  }, [onLoadMore]);

  return sentinelRef;
}
