import server from "./server.json";

/** A localhost page uses the local authority even in a production build. */
export function serverAddress(location: Pick<Location, "hostname" | "search">): string {
  if (import.meta.env.DEV) {
    const override = new URLSearchParams(location.search).get("server");
    if (override) return override.replace(/\/$/, "");
  }
  if (["localhost", "127.0.0.1", "[::1]"].includes(location.hostname))
    return "http://127.0.0.1:8787";
  return server.address;
}
