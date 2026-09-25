import type { ProxyKind } from "./proxies";

/** Which primitive stands in for a unit kind. Presentation only. */
export function proxyForUnit(kind: string): ProxyKind {
  switch (kind) {
    case "tank":
      return "tank";
    case "supply":
      return "supply";
    default:
      return "infantry";
  }
}

export const SIDE_COLORS = {
  blue: [0.55, 0.7, 1.0],
  red: [1.0, 0.55, 0.5],
} as const;
