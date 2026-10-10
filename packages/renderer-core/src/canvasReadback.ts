/** The canvas texture `texture`, drawn in this task and not yet presented,
 *  as a PNG of what the player saw: its colour bytes in RGBA order, opaque.
 *  The copy is queued at once, so it reads this frame; mapping and encoding
 *  finish later. The canvas must be configured with `COPY_SRC` usage. */
export function readCanvasPng(device: GPUDevice, texture: GPUTexture): Promise<Blob> {
  const { width, height, format } = texture;
  if (format !== "bgra8unorm" && format !== "rgba8unorm")
    return Promise.reject(new Error(`cannot read back a ${format} canvas`));
  // Copies to a buffer lay rows out at a multiple of 256 bytes.
  const stride = Math.ceil((width * 4) / 256) * 256;
  const buffer = device.createBuffer({
    label: "canvas-readback",
    size: stride * height,
    usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
  });
  const encoder = device.createCommandEncoder({ label: "canvas-readback" });
  encoder.copyTextureToBuffer({ texture }, { buffer, bytesPerRow: stride }, { width, height });
  device.queue.submit([encoder.finish()]);
  return (async () => {
    const pixels = new Uint8ClampedArray(width * height * 4);
    try {
      await buffer.mapAsync(GPUMapMode.READ);
      const rows = new Uint8Array(buffer.getMappedRange());
      const [r, b] = format === "bgra8unorm" ? [2, 0] : [0, 2];
      for (let y = 0; y < height; y++)
        for (let x = 0; x < width; x++) {
          const from = y * stride + x * 4;
          const to = (y * width + x) * 4;
          pixels[to] = rows[from + r];
          pixels[to + 1] = rows[from + 1];
          pixels[to + 2] = rows[from + b];
          pixels[to + 3] = 255;
        }
      buffer.unmap();
    } finally {
      buffer.destroy();
    }
    const canvas = new OffscreenCanvas(width, height);
    canvas.getContext("2d")!.putImageData(new ImageData(pixels, width, height), 0, 0);
    return canvas.convertToBlob({ type: "image/png" });
  })();
}
