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

// CPU mirror of the WGSL HLG OETF and tonemap math for verification
describe("HLG OETF math", () => {
  // HLG OETF per ARIB STD-B67: maps scene-linear E [0, 1] to HLG [0, 1]
  function hlgOetf(e: number): number {
    const a = 0.17883277;
    const b = 0.28466892;
    const c = 0.55991073;
    if (e <= 1.0 / 12.0) {
      return Math.sqrt(3.0 * e);
    }
    return a * Math.log(12.0 * e - b) + c;
  }

  // BT.2020 luminance
  function luma2020(rgb: [number, number, number]): number {
    return 0.2627 * rgb[0] + 0.678 * rgb[1] + 0.0593 * rgb[2];
  }

  // Inverse OOTF: display-linear (1.0 = 203 nits) -> scene-linear E
  // For HLG with 1000-nit peak display and system gamma 1.2
  function inverseOotf(displayLinear: [number, number, number]): [number, number, number] {
    // Scale to [0, 1] range where 1.0 = 1000 nits
    const ed: [number, number, number] = [
      displayLinear[0] * (203.0 / 1000.0),
      displayLinear[1] * (203.0 / 1000.0),
      displayLinear[2] * (203.0 / 1000.0),
    ];
    const yd = Math.max(luma2020(ed), 1e-6);
    const exp = (1.0 - 1.2) / 1.2;
    const scale = Math.pow(yd, exp);
    return [
      Math.max(0, Math.min(1, ed[0] * scale)),
      Math.max(0, Math.min(1, ed[1] * scale)),
      Math.max(0, Math.min(1, ed[2] * scale)),
    ];
  }

  test("reference white (display-linear 1.0) maps to HLG ~0.75", () => {
    // Reference white is display-linear [1, 1, 1] = 203 nits
    const refWhite: [number, number, number] = [1, 1, 1];
    const es = inverseOotf(refWhite);
    const hlg = es.map(hlgOetf);

    // HLG reference white should be approximately 0.75
    expect(hlg[0]).toBeCloseTo(0.75, 2);
    expect(hlg[1]).toBeCloseTo(0.75, 2);
    expect(hlg[2]).toBeCloseTo(0.75, 2);
  });

  test("1000-nit peak maps to HLG ~1.0", () => {
    // Peak white is display-linear ~4.93 (1000/203 nits)
    const peakWhite: [number, number, number] = [4.93, 4.93, 4.93];
    const es = inverseOotf(peakWhite);
    const hlg = es.map(hlgOetf);

    // Should be close to 1.0
    expect(hlg[0]).toBeCloseTo(1.0, 1);
  });

  test("black maps to HLG 0", () => {
    const black: [number, number, number] = [0, 0, 0];
    const es = inverseOotf(black);
    const hlg = es.map(hlgOetf);

    expect(hlg[0]).toBeCloseTo(0, 5);
    expect(hlg[1]).toBeCloseTo(0, 5);
    expect(hlg[2]).toBeCloseTo(0, 5);
  });

  test("midtone 0.5 maps to HLG < 0.75", () => {
    // 18% grey in display-linear is approximately 0.18
    const midGrey: [number, number, number] = [0.18, 0.18, 0.18];
    const es = inverseOotf(midGrey);
    const hlg = es.map(hlgOetf);

    // Should be less than reference white
    expect(hlg[0]).toBeLessThan(0.75);
    expect(hlg[0]).toBeGreaterThan(0);
  });

  test("HLG OETF is monotonically increasing", () => {
    let prevHlg = 0;
    for (let e = 0; e <= 1; e += 0.01) {
      const hlg = hlgOetf(e);
      expect(hlg).toBeGreaterThanOrEqual(prevHlg);
      prevHlg = hlg;
    }
  });
});
