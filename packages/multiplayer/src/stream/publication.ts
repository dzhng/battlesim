/** The publication record: tick, 16 ASCII digest bytes, step time, then f32 words. */
export interface Publication {
  tick: number;
  digest: string;
  stepMs: number;
  buffer: ArrayBuffer;
  length: number;
}

export function encodePublication(frame: Publication): Uint8Array {
  const bytes = frame.length * Float32Array.BYTES_PER_ELEMENT;
  const payload = new Uint8Array(24 + bytes);
  const view = new DataView(payload.buffer);
  view.setUint32(0, frame.tick, true);
  payload.set(new TextEncoder().encode(frame.digest).subarray(0, 16), 4);
  view.setFloat32(20, frame.stepMs, true);
  payload.set(new Uint8Array(frame.buffer, 0, bytes), 24);
  return payload;
}

export function decodePublication(payload: Uint8Array): Publication {
  const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
  const buffer = payload.slice(24).buffer;
  return {
    tick: view.getUint32(0, true),
    digest: new TextDecoder().decode(payload.subarray(4, 20)),
    stepMs: view.getFloat32(20, true),
    buffer,
    length: buffer.byteLength / Float32Array.BYTES_PER_ELEMENT,
  };
}
