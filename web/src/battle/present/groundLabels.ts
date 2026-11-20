// Labels pinned to points on the ground: each sits just above its point,
// centred on it, and hides when the point is off screen. The capture areas'
// flags and the roads in's cards are both these.
import { useMemo, useRef } from "react";

/** `pin(id)` is the ref for the label of `id`; `place(id, point)` moves it to
 *  a projected screen point, or hides it when there is none. */
export function useGroundLabels() {
  const nodes = useRef(new Map<string, HTMLElement>());
  return useMemo(
    () => ({
      pin: (id: string) => (node: HTMLElement | null) => {
        if (node) nodes.current.set(id, node);
        else nodes.current.delete(id);
      },
      place(id: string, point: readonly [number, number] | null) {
        const node = nodes.current.get(id);
        if (!node) return;
        node.style.display = point ? "" : "none";
        if (point)
          node.style.transform = `translate(${point[0]}px, ${point[1]}px) translate(-50%, -100%)`;
      },
    }),
    [],
  );
}
