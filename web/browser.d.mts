import type { Browser, LaunchOptions } from "playwright";

export function launchBrowser(options?: {
  args?: string[];
  launch?: LaunchOptions;
}): Promise<Browser>;
