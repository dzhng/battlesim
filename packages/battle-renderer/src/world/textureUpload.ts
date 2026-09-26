/** TypeGPU0.12.5 texture.write forwards source.buffer, ignoring a view's offset/length.
 * Keep exact buffers allocation-free; isolate packed subviews at this public API boundary. */
export function typegpuTextureBytes(view: ArrayBufferView): Uint8Array {
  const bytes = new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
  return bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength
    ? bytes
    : bytes.slice();
}
