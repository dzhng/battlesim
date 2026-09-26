// Spike 02 (throwaway): static server for the fog prototype.
const root = new URL(".", import.meta.url).pathname;
const server = Bun.serve({
  port: Number(process.env.PORT ?? 5802),
  async fetch(req) {
    let path = new URL(req.url).pathname;
    if (path === "/") return Response.redirect("/web/index.html", 302);
    const file = Bun.file(root + path.slice(1));
    if (!(await file.exists())) return new Response("not found", { status: 404 });
    return new Response(file, { headers: { "cache-control": "no-store" } });
  },
});
console.log(`http://localhost:${server.port}/`);
