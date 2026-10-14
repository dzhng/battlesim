// A small committed picture gate for the test-only low graphics profile. The
// full scenes own interaction and resource lifecycle; this fixture only proves
// the cheaper renderer still draws authored compositions across terrain,
// scenery and a model close-up.
export async function run(ctx) {
  const page = await ctx.newPage();
  const origin = new URL(ctx.url).origin;
  const quality = ctx.graphicsQuality ?? "low";
  const shots = [
    [ctx.url, "foundation"],
    [`${origin}/labs/geometry?map=geometry&test-quality=${quality}`, "geometry"],
    [`${origin}/labs/river?test-quality=${quality}`, "river"],
  ];
  for (const [url, name] of shots) {
    await ctx.openLab(page, url);
    const frame = await ctx.matchBaseline(page, name);
    ctx.check(`low graphics draws an opaque ${name} frame`, frame.length > 1000);
  }
}
