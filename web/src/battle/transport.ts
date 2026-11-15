/// <reference lib="es2024.arraybuffer" />
/** WASM calls copy synchronously. Release an exclusively owned transport buffer
 * after its last reader; cached textures and borrowed views keep their owners. */
export function releaseTransport(bytes: Uint8Array<ArrayBuffer>): void {
  bytes.buffer.transfer(0);
}
