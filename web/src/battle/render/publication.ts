/** Frozen records for the temporary renderer comparison controls. */
import type { PresentedFrame } from "../sim/presentation";

/** Preserve raw words, including integer payloads sharing a float view. The
 * caller must copy before any presentation call or awaited asset admission. */
export function copyPresentationFrame(frame: PresentedFrame): PresentedFrame {
  const { header, ...sections } = frame;
  const copied = Object.fromEntries(
    Object.entries(sections).map(([name, section]) => {
      const u32 = Uint32Array.from(section.u32);
      return [name, { count: section.count, u32, f32: new Float32Array(u32.buffer) }];
    }),
  );
  // Object.fromEntries loses the generated record keys; every source section
  // is copied mechanically so additions to the canonical ABI are included.
  return { header: Uint32Array.from(header), ...copied } as PresentedFrame;
}
