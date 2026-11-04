export function hdrEncodeArgs(options: {
  framePattern: string;
  audio: string | null;
  output: string;
  fps: number | string;
  width: number;
  height: number;
  hevc?: boolean;
}): string[];
export function hdrProbeArgs(file: string): string[];
export function hlgTagErrors(probe: unknown): string[];
