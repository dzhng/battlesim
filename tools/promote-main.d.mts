export function promoteMain(options?: {
  yes?: boolean;
  runner?: (command: string, args: string[]) => { status: number | null; stdout: string };
  report?: (line: string) => void;
}): number;
/** The site as a reader reaches it. */
export const productionSite: string;
