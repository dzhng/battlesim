import type { modelVertexLayout, modelRecordLayout, modelLayout } from "./modelLayer";
import type { CardGroup } from "./impostorCards";

/** Anything a bound pipeline can draw models through. */
export interface Drawable3 {
  with(layout: typeof modelVertexLayout, buffer: GPUBuffer): Drawable3;
  with(layout: typeof modelRecordLayout, buffer: GPUBuffer): Drawable3;
  with(layout: typeof modelLayout, group: GPUBindGroup): Drawable3;
  withIndexBuffer(
    buffer: GPUBuffer,
    format: GPUIndexFormat,
  ): {
    drawIndexed(
      count: number,
      instances: number,
      first: number,
      baseVertex: number,
      firstInstance: number,
    ): void;
  };
}
/** Anything a bound card pipeline can draw through. */
export interface CardDrawable {
  with(layout: typeof modelRecordLayout, buffer: GPUBuffer): CardDrawable;
  with(group: CardGroup): CardDrawable;
  draw(vertices: number, instances: number, firstVertex: number, firstInstance: number): void;
}
