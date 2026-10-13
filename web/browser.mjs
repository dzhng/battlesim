// The browser every WebGPU check draws in: scenes, tutorial shots, model
// sheets and the browser tests that need a GPU.
//
// It is Playwright's Chromium, except inside WSL, where it is headless Windows
// Chrome, started and stopped per run; WEBGPU_BROWSER=local keeps Chromium.
//
// WSL has no Vulkan driver for the host GPU, so its Chromium draws WebGPU on
// SwiftShader, a software adapter whose limits (256 texture-array layers) a
// battle exceeds. Windows Chrome draws on the real GPU through D3D12. WSL's
// mirrored networking lets it load this side's 127.0.0.1 servers and lets
// Playwright attach to it.
import { spawn, spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { randomUUID } from "node:crypto";

const WEBGPU_FLAGS = ["--enable-unsafe-webgpu", "--enable-features=WebGPU"];

const WINDOWS_CHROME = "/mnt/c/Program Files/Google/Chrome/Application/chrome.exe";

/** Playwright passes these to a browser it launches; one started by hand
 *  needs them too, or a page in a window Windows counts as hidden stops
 *  drawing frames and a lab takes a minute to become ready. */
const UNTHROTTLED = [
  "--disable-background-timer-throttling",
  "--disable-backgrounding-occluded-windows",
  "--disable-renderer-backgrounding",
];

/** Which browser the setting and this machine select. */
export function browserChoice(setting, { wsl, mirrored, windowsChrome }) {
  if (setting && setting !== "local")
    throw new Error(`WEBGPU_BROWSER=${setting}: the one setting is local`);
  if (setting === "local" || !wsl) return { kind: "local" };
  const unreachable = !mirrored
    ? "WSL networking is not mirrored (set networkingMode=mirrored in .wslconfig)"
    : !windowsChrome
      ? `there is no Windows Chrome at ${WINDOWS_CHROME}`
      : null;
  if (!unreachable) return { kind: "windows" };
  return { kind: "local", note: `WebGPU runs on SwiftShader in WSL's Chromium: ${unreachable}` };
}

function machine() {
  const wsl = Boolean(process.env.WSL_DISTRO_NAME);
  const mode = wsl ? spawnSync("wslinfo", ["--networking-mode"], { encoding: "utf8" }) : null;
  return {
    wsl,
    mirrored: mode?.stdout?.trim() === "mirrored",
    windowsChrome: existsSync(WINDOWS_CHROME),
  };
}

/** A WebGPU browser for this run. `args` are extra Chromium flags; `launch`
 *  holds Playwright launch options for local Chromium. Closing the browser
 *  stops what this started. */
export async function launchBrowser({ args = [], launch = {} } = {}) {
  const { chromium } = await import("playwright");
  const choice = browserChoice(process.env.WEBGPU_BROWSER, machine());
  if (choice.note) console.warn(choice.note);
  if (choice.kind === "local")
    return chromium.launch({ channel: "chromium", ...launch, args: [...WEBGPU_FLAGS, ...args] });
  return launchWindowsChrome(chromium, args);
}

async function launchWindowsChrome(chromium, args, attempt = 1) {
  const temp = spawnSync("cmd.exe", ["/c", "echo", "%TEMP%"], {
    cwd: "/mnt/c",
    encoding: "utf8",
  }).stdout.trim();
  const profileWindows = `${temp}\\battlesim-webgpu-${randomUUID()}`;
  const profile = spawnSync("wslpath", ["-u", profileWindows], { encoding: "utf8" }).stdout.trim();
  // Port 0: Chrome picks a free port and writes it to DevToolsActivePort, so
  // concurrent runs never share a browser.
  const child = spawn(
    WINDOWS_CHROME,
    [
      "--headless=new",
      "--remote-debugging-port=0",
      `--user-data-dir=${profileWindows}`,
      "--no-first-run",
      "--no-default-browser-check",
      ...UNTHROTTLED,
      ...WEBGPU_FLAGS,
      ...args,
      "about:blank",
    ],
    { stdio: "ignore" },
  );
  // Killing the WSL side of an interop process leaves chrome.exe running, so
  // a run that ends without closing (a crash, Ctrl-C) stops it from Windows,
  // found by its profile, which no other browser shares.
  const stop = () => {
    process.off("exit", stop);
    for (const signal of ["SIGINT", "SIGTERM"]) process.off(signal, onSignal);
    spawnSync("powershell.exe", [
      "-NoProfile",
      "-Command",
      `Get-CimInstance Win32_Process -Filter "Name='chrome.exe'" | ` +
        `Where-Object { $_.CommandLine -like '*${profileWindows}*' } | ` +
        `ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }; ` +
        `Start-Sleep -Milliseconds 500; ` +
        `Remove-Item -Recurse -Force -ErrorAction SilentlyContinue '${profileWindows}'`,
    ]);
  };
  const onSignal = (signal) => {
    stop();
    process.kill(process.pid, signal);
  };
  process.once("exit", stop);
  for (const signal of ["SIGINT", "SIGTERM"]) process.once(signal, onSignal);

  let browser;
  try {
    const port = await devToolsPort(`${profile}/DevToolsActivePort`, child);
    // Now and then chrome.exe exits cleanly before it listens; a fresh start
    // comes up.
    if (!port && attempt < 3) {
      stop();
      return launchWindowsChrome(chromium, args, attempt + 1);
    }
    if (!port) throw new Error(`chrome.exe exited with ${child.exitCode}, ${attempt} times`);
    browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  } catch (error) {
    stop();
    throw new Error(`Windows Chrome did not start: ${error.message}`, { cause: error });
  }
  // Closing an attached browser only disconnects; this one is ours to stop.
  const disconnect = browser.close.bind(browser);
  browser.close = async () => {
    await disconnect().catch(() => {});
    stop();
  };
  return browser;
}

/** The port Chrome listens on, or null if it exited first. */
async function devToolsPort(file, child) {
  for (let waited = 0; waited < 30_000; waited += 100) {
    if (child.exitCode !== null) return null;
    try {
      const port = readFileSync(file, "utf8").split("\n")[0].trim();
      if (port) return port;
    } catch {}
    await new Promise((wait) => setTimeout(wait, 100));
  }
  throw new Error("no DevToolsActivePort after 30 s");
}
