// Cinematic capture scene: runs the capture route and waits for recording to complete.
// Use: bun run --cwd web scene -- capture
// The scene writes the recorded video to throwaway/evidence/capture/

import { writeFile } from "node:fs/promises";

export async function run(ctx) {
  const page = await ctx.newPage({
    viewport: { width: 1920, height: 1080 },
    deviceScaleFactor: 1,
    allowErrors: false,
  });

  try {
    const url = ctx.urlFor(ctx.url, { capture: "default" });
    await page.goto(url);

    // Wait for capture to complete (or error)
    await page.waitForFunction(
      () => {
        const capture = window.__capture;
        return capture?.state === "complete" || capture?.state === "error" || window.__lab?.error;
      },
      undefined,
      { timeout: 120_000 },
    );

    const result = await page.evaluate(() => window.__capture);

    if (result?.state === "error") {
      ctx.check("capture completed", false, result.error);
      return;
    }

    if (result?.state !== "complete") {
      const labError = await page.evaluate(() => window.__lab?.error);
      ctx.check("capture completed", false, labError ?? "unknown error");
      return;
    }

    // Download the blob
    const base64 = await page.evaluate(async () => {
      const capture = window.__capture;
      if (!capture?.blob) return null;
      const buffer = await capture.blob.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      let binary = "";
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
      }
      return btoa(binary);
    });

    if (base64) {
      const videoData = Buffer.from(base64, "base64");
      const outputPath = ctx.evidencePath(result.filename);
      await writeFile(outputPath, videoData);
      ctx.check("video saved", true, `${videoData.length} bytes to ${outputPath}`);
    } else {
      ctx.check("video saved", false, "could not extract blob");
    }
  } finally {
    await page.close();
  }
}
