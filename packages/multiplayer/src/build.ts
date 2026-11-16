import type { BuildIdentity } from "./wire.ts";

/** A page's source commit is provenance, never admission. */
export function sameBuild(page: BuildIdentity | undefined, server: BuildIdentity): boolean {
  return page?.sim === server.sim && page?.data === server.data && page?.wire === server.wire;
}
