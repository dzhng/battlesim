// @vitest-environment node
import { Buffer } from "node:buffer";
import { expect, test } from "vitest";
import { decodeContainer, encodeContainer } from "@packages/scene-assets/src/codec";

test("container readers preserve distinct typed views from Buffer-backed input", () => {
  const content = { a: new Uint8Array([11, 22, 33]), b: new Float32Array([1.25, -2.5]) };
  const encoded = encodeContainer(123, 1, content);
  const storage = Buffer.concat([Buffer.alloc(7), Buffer.from(encoded), Buffer.alloc(3)]);
  const input = storage.subarray(7, 7 + encoded.length);
  expect(decodeContainer(input, 123, 1, "fixture")).toEqual(content);
});
