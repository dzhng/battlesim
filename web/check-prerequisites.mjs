import { spawnSync } from "node:child_process";

const missing = ["ffmpeg", "ffprobe"].filter(
  (tool) => spawnSync(tool, ["-version"], { stdio: "ignore" }).status !== 0,
);

if (missing.length) {
  const install =
    process.platform === "darwin"
      ? "brew install ffmpeg"
      : process.env.WSL_DISTRO_NAME
        ? "sudo apt-get update && sudo apt-get install -y ffmpeg"
        : process.platform === "linux"
          ? "sudo apt-get update && sudo apt-get install -y ffmpeg"
          : "install ffmpeg and ffprobe with your platform package manager";
  throw new Error(
    `Missing ${missing.join(" and ")}. Install them with: ${install}. Then rerun bun run setup.`,
  );
}

console.log("Audio prerequisites ready: ffmpeg and ffprobe");
