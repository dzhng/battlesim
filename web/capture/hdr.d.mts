export const HLG_HEVC: string[];
export const HLG_H264: string[];
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
export function verifyHdrMetadata(probe: unknown): { valid: boolean; errors: string[] };
