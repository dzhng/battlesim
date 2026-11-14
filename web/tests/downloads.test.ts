// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
import { countedFetch, downloads } from "../src/downloads";

afterEach(() => vi.unstubAllGlobals());

/** A response streaming `chunks` (bytes each), one when `release` is called. */
function streamed(chunks: number[]) {
  let push: () => void = () => {};
  let next = 0;
  const body = new ReadableStream<Uint8Array>({
    pull(controller) {
      return new Promise<void>((resolve) => {
        push = () => {
          if (next < chunks.length) controller.enqueue(new Uint8Array(chunks[next++]));
          else controller.close();
          resolve();
        };
      });
    },
  });
  const total = chunks.reduce((a, b) => a + b, 0);
  const response = new Response(body, { headers: { "content-length": String(total) } });
  return { response, release: () => push() };
}

const tick = () => new Promise((r) => setTimeout(r, 0));

test("the bytes of a download count up as its body arrives, against its announced size, and the burst resets when every download is done", async () => {
  const a = streamed([600, 400]);
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => a.response),
  );
  const body = countedFetch("/bundle.bin").then((r) => r.arrayBuffer());
  await tick();
  expect(downloads.get()).toMatchObject({ active: 1, loaded: 0, total: 1000 });
  a.release();
  await tick();
  expect(downloads.get()).toMatchObject({ active: 1, loaded: 600, total: 1000 });
  a.release();
  await tick();
  a.release();
  // The body reads whole, unchanged.
  expect((await body).byteLength).toBe(1000);
  expect(downloads.get()).toMatchObject({ active: 0, loaded: 0, total: 0 });
});

/** A response that sends 300 of an announced 1000 bytes, then loses its
 *  connection the way a browser reports it (a TypeError). */
function dropped() {
  let sent = false;
  const body = new ReadableStream<Uint8Array>({
    pull(controller) {
      if (sent) controller.error(new TypeError("Load failed"));
      else controller.enqueue(new Uint8Array(300));
      sent = true;
    },
  });
  return new Response(body, { headers: { "content-length": "1000" } });
}

test.each([
  ["its connection drops mid-body", (r: Response) => r.arrayBuffer()],
  ["its reader cancels the body", (r: Response) => r.body!.cancel()],
])("a download ends, and the burst resets, when %s", async (_, end) => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => dropped()),
  );
  await end(await countedFetch("/bundle.bin")).catch(() => {});
  await tick();
  expect(downloads.get()).toMatchObject({ active: 0, loaded: 0, total: 0 });
});

test("retries a cache write failure without using the browser cache", async () => {
  const response = new Response(new Uint8Array([1, 2, 3]));
  const fetcher = vi
    .fn()
    .mockRejectedValueOnce(new TypeError("Failed to fetch"))
    .mockResolvedValueOnce(response);
  vi.stubGlobal("fetch", fetcher);

  expect(new Uint8Array(await (await countedFetch("/bundle.bin")).arrayBuffer())).toEqual(
    new Uint8Array([1, 2, 3]),
  );
  expect(fetcher).toHaveBeenNthCalledWith(2, "/bundle.bin", { cache: "no-store" });
});
