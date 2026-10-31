/**
 * @vitest-environment node
 */
/**
 * GPU-free end-to-end test for HLG video encoding.
 *
 * Creates synthetic HLG-encoded EXR frames with known values:
 * - Reference white (HLG ~0.75, 203 nits display)
 * - Peak white (HLG ~1.0, 1000 nits display)
 * - Midtone (HLG ~0.5)
 *
 * Runs the real ffmpeg encode via hdr.mjs, then verifies:
 * - ffprobe tags: HEVC Main 10, BT.2020 primaries/matrix, arib-std-b67, tv range
 * - Decoded values round-trip correctly
 */
import { expect, test, describe, beforeAll, afterAll } from "vitest";
import { spawn, execSync } from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import { hdrEncodeArgs, hdrProbeArgs, verifyHdrMetadata } from "../capture/hdr.mjs";

let tempDir: string;

function checkFfmpeg(): boolean {
  try {
    execSync("ffmpeg -version", { stdio: "ignore" });
    execSync("ffprobe -version", { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

const ffmpegAvailable = checkFfmpeg();

function runCommand(cmd: string, args: string[]): Promise<{ code: number; stdout: string; stderr: string }> {
  return new Promise((resolve) => {
    const proc = spawn(cmd, args, { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    proc.stdout.on("data", (d) => (stdout += d.toString()));
    proc.stderr.on("data", (d) => (stderr += d.toString()));
    proc.on("close", (code) => resolve({ code: code ?? 1, stdout, stderr }));
  });
}

function floatToHalf(f: number): number {
  const floatView = new Float32Array(1);
  const int32View = new Int32Array(floatView.buffer);
  floatView[0] = f;
  const x = int32View[0];
  const sign = (x >> 31) & 0x0001;
  const exp = (x >> 23) & 0x00ff;
  const frac = x & 0x007fffff;

  if (exp === 0) {
    return sign << 15;
  } else if (exp === 0xff) {
    return (sign << 15) | 0x7c00 | (frac ? 1 : 0);
  }

  const newExp = exp - 127 + 15;
  if (newExp >= 31) {
    return (sign << 15) | 0x7c00;
  } else if (newExp <= 0) {
    return sign << 15;
  }
  return (sign << 15) | (newExp << 10) | (frac >> 13);
}

function writeMinimalExr(filePath: string, width: number, height: number, rgb: [number, number, number]): void {
  const channels = ["B", "G", "R"];
  const channelData = new Map<string, number>();
  channelData.set("R", rgb[0]);
  channelData.set("G", rgb[1]);
  channelData.set("B", rgb[2]);

  const stringBytes = (s: string) => Buffer.from(s + "\0", "ascii");
  const header: Buffer[] = [];
  const magic = Buffer.alloc(4);
  magic.writeUInt32LE(20000630, 0);
  header.push(magic);

  const version = Buffer.alloc(4);
  version.writeUInt32LE(2, 0);
  header.push(version);

  const addAttr = (name: string, type: string, value: Buffer) => {
    header.push(stringBytes(name));
    header.push(stringBytes(type));
    const sizeB = Buffer.alloc(4);
    sizeB.writeUInt32LE(value.length, 0);
    header.push(sizeB);
    header.push(value);
  };

  const chlistData: Buffer[] = [];
  for (const ch of channels) {
    chlistData.push(stringBytes(ch));
    const chInfo = Buffer.alloc(16);
    chInfo.writeUInt32LE(1, 0); // half
    chInfo.writeUInt32LE(0, 4); // pLinear
    chInfo.writeInt32LE(1, 8);  // xSampling
    chInfo.writeInt32LE(1, 12); // ySampling
    chlistData.push(chInfo);
  }
  chlistData.push(Buffer.from([0]));
  addAttr("channels", "chlist", Buffer.concat(chlistData));

  const compressionB = Buffer.alloc(1);
  compressionB.writeUInt8(0, 0);
  addAttr("compression", "compression", compressionB);

  const box = Buffer.alloc(16);
  box.writeInt32LE(0, 0);
  box.writeInt32LE(0, 4);
  box.writeInt32LE(width - 1, 8);
  box.writeInt32LE(height - 1, 12);
  addAttr("dataWindow", "box2i", box);
  addAttr("displayWindow", "box2i", box);

  const lineOrderB = Buffer.alloc(1);
  lineOrderB.writeUInt8(0, 0);
  addAttr("lineOrder", "lineOrder", lineOrderB);

  const parB = Buffer.alloc(4);
  parB.writeFloatLE(1.0, 0);
  addAttr("pixelAspectRatio", "float", parB);

  const screenB = Buffer.alloc(8);
  screenB.writeFloatLE(0, 0);
  screenB.writeFloatLE(0, 4);
  addAttr("screenWindowCenter", "v2f", screenB);
  const screenWB = Buffer.alloc(4);
  screenWB.writeFloatLE(1.0, 0);
  addAttr("screenWindowWidth", "float", screenWB);

  header.push(Buffer.from([0]));

  const headerBuf = Buffer.concat(header);
  const scanlineSize = width * 3 * 2;
  const offsetTableSize = height * 8;
  const offsetTableStart = headerBuf.length;
  const dataStart = offsetTableStart + offsetTableSize;

  const offsetTable = Buffer.alloc(offsetTableSize);
  for (let y = 0; y < height; y++) {
    const offset = dataStart + y * (4 + 4 + scanlineSize);
    offsetTable.writeBigUInt64LE(BigInt(offset), y * 8);
  }

  const pixelData: Buffer[] = [];
  for (let y = 0; y < height; y++) {
    const lineHeader = Buffer.alloc(8);
    lineHeader.writeInt32LE(y, 0);
    lineHeader.writeInt32LE(scanlineSize, 4);
    pixelData.push(lineHeader);

    for (const ch of channels) {
      const val = channelData.get(ch) ?? 0;
      const half = floatToHalf(val);
      const rowData = Buffer.alloc(width * 2);
      for (let x = 0; x < width; x++) {
        rowData.writeUInt16LE(half, x * 2);
      }
      pixelData.push(rowData);
    }
  }

  const file = Buffer.concat([headerBuf, offsetTable, ...pixelData]);
  fs.writeFileSync(filePath, file);
}

beforeAll(() => {
  if (ffmpegAvailable) {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "hdr-e2e-"));
  }
});

afterAll(() => {
  if (tempDir && fs.existsSync(tempDir)) {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

describe("HDR encode end-to-end", () => {
  test.skipIf(!ffmpegAvailable)("encodes synthetic EXR frames to HLG HEVC with correct metadata", async () => {
    const width = 64;
    const height = 64;
    const numFrames = 3;
    const fps = 30;

    const hlgValues: [number, number, number][] = [
      [0.75, 0.75, 0.75],
      [1.0, 1.0, 1.0],
      [0.5, 0.5, 0.5],
    ];

    for (let i = 0; i < numFrames; i++) {
      const framePath = path.join(tempDir, `frame-${String(i).padStart(6, "0")}.exr`);
      writeMinimalExr(framePath, width, height, hlgValues[i]);
    }

    const outputPath = path.join(tempDir, "output.mp4");
    const encodeArgs = hdrEncodeArgs({
      framePattern: path.join(tempDir, "frame-%06d.exr"),
      audio: null,
      output: outputPath,
      fps,
      width,
      height,
      hevc: true,
      startNumber: 0,
    });

    const encodeResult = await runCommand("ffmpeg", encodeArgs);
    expect(encodeResult.code).toBe(0);
    expect(fs.existsSync(outputPath)).toBe(true);

    const probeArgs = hdrProbeArgs(outputPath);
    const probeResult = await runCommand("ffprobe", probeArgs);
    expect(probeResult.code).toBe(0);

    const probe = JSON.parse(probeResult.stdout);
    const { valid, errors } = verifyHdrMetadata(probe);
    expect(errors).toEqual([]);
    expect(valid).toBe(true);

    const stream = probe.streams[0];
    expect(stream.codec_name).toBe("hevc");
    expect(stream.profile).toBe("Main 10");
    expect(stream.pix_fmt).toBe("yuv420p10le");
    expect(stream.color_primaries).toBe("bt2020");
    expect(stream.color_space).toBe("bt2020nc");
    expect(stream.color_transfer).toBe("arib-std-b67");
    expect(stream.color_range).toBe("tv");
  }, 30000);

  test.skipIf(!ffmpegAvailable)("reference white encodes to approximately HLG 0.75 in YUV", async () => {
    const width = 64;
    const height = 64;
    const fps = 1;

    const framePath = path.join(tempDir, "ref-white-000000.exr");
    writeMinimalExr(framePath, width, height, [0.75, 0.75, 0.75]);

    const outputPath = path.join(tempDir, "ref-white.mp4");
    const encodeArgs = hdrEncodeArgs({
      framePattern: path.join(tempDir, "ref-white-%06d.exr"),
      audio: null,
      output: outputPath,
      fps,
      width,
      height,
      hevc: true,
      startNumber: 0,
    });

    const encodeResult = await runCommand("ffmpeg", encodeArgs);
    expect(encodeResult.code).toBe(0);

    const rawPath = path.join(tempDir, "ref-white.yuv");
    const decodeResult = await runCommand("ffmpeg", [
      "-y", "-hide_banner", "-loglevel", "error",
      "-i", outputPath,
      "-vframes", "1",
      "-f", "rawvideo",
      "-pix_fmt", "yuv420p10le",
      rawPath,
    ]);
    expect(decodeResult.code).toBe(0);

    const yuvData = fs.readFileSync(rawPath);
    const yPlaneSize = width * height * 2;
    const yValues: number[] = [];
    for (let i = 0; i < yPlaneSize; i += 2) {
      yValues.push(yuvData.readUInt16LE(i));
    }

    const avgY = yValues.reduce((a, b) => a + b, 0) / yValues.length;
    const normalizedY = avgY / 1023;

    expect(normalizedY).toBeGreaterThan(0.65);
    expect(normalizedY).toBeLessThan(0.85);
  }, 30000);

  test.skipIf(!ffmpegAvailable)("peak white encodes higher than reference white", async () => {
    const width = 64;
    const height = 64;
    const fps = 1;

    const refFramePath = path.join(tempDir, "peak-ref-000000.exr");
    writeMinimalExr(refFramePath, width, height, [0.75, 0.75, 0.75]);

    const peakFramePath = path.join(tempDir, "peak-bright-000000.exr");
    writeMinimalExr(peakFramePath, width, height, [1.0, 1.0, 1.0]);

    const refOutputPath = path.join(tempDir, "peak-ref.mp4");
    const peakOutputPath = path.join(tempDir, "peak-bright.mp4");

    await runCommand("ffmpeg", hdrEncodeArgs({
      framePattern: path.join(tempDir, "peak-ref-%06d.exr"),
      audio: null, output: refOutputPath,
      fps, width, height, hevc: true, startNumber: 0,
    }));

    await runCommand("ffmpeg", hdrEncodeArgs({
      framePattern: path.join(tempDir, "peak-bright-%06d.exr"),
      audio: null, output: peakOutputPath,
      fps, width, height, hevc: true, startNumber: 0,
    }));

    const refYuvPath = path.join(tempDir, "peak-ref.yuv");
    const peakYuvPath = path.join(tempDir, "peak-bright.yuv");

    await runCommand("ffmpeg", [
      "-y", "-hide_banner", "-loglevel", "error",
      "-i", refOutputPath, "-vframes", "1",
      "-f", "rawvideo", "-pix_fmt", "yuv420p10le", refYuvPath,
    ]);

    await runCommand("ffmpeg", [
      "-y", "-hide_banner", "-loglevel", "error",
      "-i", peakOutputPath, "-vframes", "1",
      "-f", "rawvideo", "-pix_fmt", "yuv420p10le", peakYuvPath,
    ]);

    const getAvgY = (yuvPath: string): number => {
      const data = fs.readFileSync(yuvPath);
      const yPlaneSize = width * height * 2;
      let sum = 0;
      for (let i = 0; i < yPlaneSize; i += 2) {
        sum += data.readUInt16LE(i);
      }
      return sum / (width * height);
    };

    const refY = getAvgY(refYuvPath);
    const peakY = getAvgY(peakYuvPath);

    expect(peakY).toBeGreaterThan(refY);
    expect(peakY / 1023).toBeGreaterThan(0.9);
  }, 30000);

  test.skipIf(!ffmpegAvailable)("midtone encodes lower than reference white", async () => {
    const width = 64;
    const height = 64;
    const fps = 1;

    const refFramePath = path.join(tempDir, "mid-ref-000000.exr");
    writeMinimalExr(refFramePath, width, height, [0.75, 0.75, 0.75]);

    const midFramePath = path.join(tempDir, "mid-dark-000000.exr");
    writeMinimalExr(midFramePath, width, height, [0.5, 0.5, 0.5]);

    const refOutputPath = path.join(tempDir, "mid-ref.mp4");
    const midOutputPath = path.join(tempDir, "mid-dark.mp4");

    await runCommand("ffmpeg", hdrEncodeArgs({
      framePattern: path.join(tempDir, "mid-ref-%06d.exr"),
      audio: null, output: refOutputPath,
      fps, width, height, hevc: true, startNumber: 0,
    }));

    await runCommand("ffmpeg", hdrEncodeArgs({
      framePattern: path.join(tempDir, "mid-dark-%06d.exr"),
      audio: null, output: midOutputPath,
      fps, width, height, hevc: true, startNumber: 0,
    }));

    const refYuvPath = path.join(tempDir, "mid-ref.yuv");
    const midYuvPath = path.join(tempDir, "mid-dark.yuv");

    await runCommand("ffmpeg", [
      "-y", "-hide_banner", "-loglevel", "error",
      "-i", refOutputPath, "-vframes", "1",
      "-f", "rawvideo", "-pix_fmt", "yuv420p10le", refYuvPath,
    ]);

    await runCommand("ffmpeg", [
      "-y", "-hide_banner", "-loglevel", "error",
      "-i", midOutputPath, "-vframes", "1",
      "-f", "rawvideo", "-pix_fmt", "yuv420p10le", midYuvPath,
    ]);

    const getAvgY = (yuvPath: string): number => {
      const data = fs.readFileSync(yuvPath);
      const yPlaneSize = width * height * 2;
      let sum = 0;
      for (let i = 0; i < yPlaneSize; i += 2) {
        sum += data.readUInt16LE(i);
      }
      return sum / (width * height);
    };

    const refY = getAvgY(refYuvPath);
    const midY = getAvgY(midYuvPath);

    expect(midY).toBeLessThan(refY);
    expect(midY / 1023).toBeLessThan(0.75);
    expect(midY / 1023).toBeGreaterThan(0.3);
  }, 30000);
});
