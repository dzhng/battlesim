import { expect, test, describe } from "vitest";
import {
  hdrEncodeArgs,
  hdrProbeArgs,
  verifyHdrMetadata,
  HLG_HEVC,
  HLG_H264,
} from "../capture/hdr.mjs";

describe("HDR encode arguments", () => {
  test("generates correct HEVC HLG encoding command", () => {
    const args = hdrEncodeArgs({
      framePattern: "frames/plate-%04d.exr",
      audio: "sound.wav",
      output: "output.mp4",
      fps: 60,
      width: 1920,
      height: 1080,
      hevc: true,
    });

    expect(args).toContain("-c:v");
    expect(args).toContain("libx265");
    expect(args).toContain("-profile:v");
    expect(args).toContain("main10");
    expect(args).toContain("-pix_fmt");
    expect(args).toContain("yuv420p10le");
    expect(args).toContain("-color_primaries");
    expect(args).toContain("bt2020");
    expect(args).toContain("-colorspace");
    expect(args).toContain("bt2020nc");
    expect(args).toContain("-color_trc");
    expect(args).toContain("arib-std-b67");
    expect(args).toContain("-color_range");
    expect(args).toContain("tv");
    expect(args).toContain("-tag:v");
    expect(args).toContain("hvc1");
    expect(args).toContain("-movflags");
    expect(args).toContain("+faststart");
    expect(args).toContain("-framerate");
    expect(args).toContain("60");
    expect(args).toContain("-i");
    expect(args).toContain("frames/plate-%04d.exr");
    expect(args).toContain("sound.wav");
    expect(args).toContain("output.mp4");
  });

  test("generates correct H.264 HLG encoding command", () => {
    const args = hdrEncodeArgs({
      framePattern: "frames/plate-%04d.exr",
      audio: "sound.wav",
      output: "output.mp4",
      fps: 30,
      width: 1920,
      height: 1080,
      hevc: false,
    });

    expect(args).toContain("-c:v");
    expect(args).toContain("libx264");
    expect(args).toContain("-profile:v");
    expect(args).toContain("high10");
    expect(args).toContain("-pix_fmt");
    expect(args).toContain("yuv420p10le");
    expect(args).toContain("-color_primaries");
    expect(args).toContain("bt2020");
    expect(args).toContain("-framerate");
    expect(args).toContain("30");
  });

  test("omits audio flags when no audio provided", () => {
    const args = hdrEncodeArgs({
      framePattern: "frames/plate-%04d.exr",
      audio: null as unknown as string,
      output: "output.mp4",
      fps: 60,
      width: 1920,
      height: 1080,
      hevc: true,
    });

    expect(args).not.toContain("sound.wav");
    expect(args).toContain("-an");
  });

  test("includes correct video filter for scaling and padding", () => {
    const args = hdrEncodeArgs({
      framePattern: "frames/plate-%04d.exr",
      audio: null as unknown as string,
      output: "output.mp4",
      fps: 60,
      width: 1920,
      height: 1080,
      hevc: true,
    });

    expect(args).toContain("-vf");
    const vfIndex = args.indexOf("-vf");
    const vfValue = args[vfIndex + 1];
    expect(vfValue).toContain("scale=1920:1080");
    expect(vfValue).toContain("pad=1920:1080");
    expect(vfValue).toContain("setsar=1");
  });

  test("includes start_number when provided", () => {
    const args = hdrEncodeArgs({
      framePattern: "frames/frame-%06d.exr",
      audio: null as unknown as string,
      output: "output.mp4",
      fps: 60,
      width: 1920,
      height: 1080,
      hevc: true,
      startNumber: 100,
    });

    expect(args).toContain("-start_number");
    expect(args).toContain("100");
  });

  test("omits start_number when 0", () => {
    const args = hdrEncodeArgs({
      framePattern: "frames/frame-%06d.exr",
      audio: null as unknown as string,
      output: "output.mp4",
      fps: 60,
      width: 1920,
      height: 1080,
      hevc: true,
      startNumber: 0,
    });

    expect(args).not.toContain("-start_number");
  });
});

describe("HDR probe arguments", () => {
  test("generates correct ffprobe command", () => {
    const args = hdrProbeArgs("test.mp4");

    expect(args).toContain("-v");
    expect(args).toContain("error");
    expect(args).toContain("-select_streams");
    expect(args).toContain("v:0");
    expect(args).toContain("-show_entries");
    expect(args).toContain("-of");
    expect(args).toContain("json");
    expect(args).toContain("test.mp4");
  });
});

describe("HDR metadata verification", () => {
  test("accepts valid HEVC HDR metadata", () => {
    const probe = {
      streams: [
        {
          codec_name: "hevc",
          profile: "Main 10",
          pix_fmt: "yuv420p10le",
          color_primaries: "bt2020",
          color_space: "bt2020nc",
          color_transfer: "arib-std-b67",
          color_range: "tv",
          width: 1920,
          height: 1080,
        },
      ],
    };

    const { valid, errors } = verifyHdrMetadata(probe);
    expect(valid).toBe(true);
    expect(errors).toHaveLength(0);
  });

  test("accepts valid H.264 HDR metadata", () => {
    const probe = {
      streams: [
        {
          codec_name: "h264",
          profile: "High 10",
          pix_fmt: "yuv420p10le",
          color_primaries: "bt2020",
          color_space: "bt2020nc",
          color_transfer: "arib-std-b67",
          color_range: "tv",
          width: 1920,
          height: 1080,
        },
      ],
    };

    const { valid, errors } = verifyHdrMetadata(probe);
    expect(valid).toBe(true);
    expect(errors).toHaveLength(0);
  });

  test("rejects SDR metadata", () => {
    const probe = {
      streams: [
        {
          codec_name: "h264",
          profile: "High",
          pix_fmt: "yuv420p",
          color_primaries: "bt709",
          color_space: "bt709",
          color_transfer: "bt709",
          color_range: "tv",
          width: 1920,
          height: 1080,
        },
      ],
    };

    const { valid, errors } = verifyHdrMetadata(probe);
    expect(valid).toBe(false);
    expect(errors.length).toBeGreaterThan(0);
    expect(errors.some((e) => e.includes("pix_fmt"))).toBe(true);
    expect(errors.some((e) => e.includes("color_primaries"))).toBe(true);
  });

  test("rejects missing stream", () => {
    const probe = { streams: [] };

    const { valid, errors } = verifyHdrMetadata(probe);
    expect(valid).toBe(false);
    expect(errors).toContain("no video stream found");
  });

  test("reports all metadata errors", () => {
    const probe = {
      streams: [
        {
          codec_name: "h264",
          pix_fmt: "yuv420p",
          color_primaries: "bt709",
          color_space: "bt709",
          color_transfer: "bt709",
          color_range: "full",
        },
      ],
    };

    const { valid, errors } = verifyHdrMetadata(probe);
    expect(valid).toBe(false);
    expect(errors.length).toBe(5);
  });
});

describe("HLG codec constants", () => {
  test("HEVC constants include required HLG parameters", () => {
    expect(HLG_HEVC).toContain("libx265");
    expect(HLG_HEVC).toContain("main10");
    expect(HLG_HEVC).toContain("yuv420p10le");
    expect(HLG_HEVC).toContain("bt2020");
    expect(HLG_HEVC).toContain("bt2020nc");
    expect(HLG_HEVC).toContain("arib-std-b67");
    expect(HLG_HEVC).toContain("tv");
    expect(HLG_HEVC).toContain("hvc1");
    expect(HLG_HEVC.some((arg) => arg.includes("repeat-headers"))).toBe(true);
  });

  test("H.264 constants include required HLG parameters", () => {
    expect(HLG_H264).toContain("libx264");
    expect(HLG_H264).toContain("high10");
    expect(HLG_H264).toContain("yuv420p10le");
    expect(HLG_H264).toContain("bt2020");
    expect(HLG_H264).toContain("bt2020nc");
    expect(HLG_H264).toContain("arib-std-b67");
    expect(HLG_H264).toContain("tv");
  });
});
