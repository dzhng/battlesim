import { spawn } from "node:child_process";
import { resolve } from "node:path";
import type {
  NativeRequest,
  NativeValidation,
  NativeReport,
  Inspection,
  SightReport,
} from "./src/protocol";
import { WorkbenchError } from "./error";
export type NativeReporter = (
  request: NativeRequest,
  signal?: AbortSignal,
) => Promise<NativeValidation | NativeReport | Inspection | SightReport>;

/** Each request owns its native process; abort waits for its exit before releasing artifacts. */
export function nativeReporter(root: string): NativeReporter {
  return (request, signal) =>
    new Promise((complete, reject) => {
      if (signal?.aborted) return reject(signal.reason ?? new Error("Request cancelled"));
      const child = spawn(
        resolve(
          root,
          process.env.CARGO_TARGET_DIR ?? "throwaway/target",
          "debug/examples/map_workbench_report",
        ),
        [],
        { cwd: root, stdio: ["pipe", "pipe", "pipe"] },
      );
      const stdout: Buffer[] = [];
      let bytes = 0,
        stderr = "",
        failure: Error | null = null;
      const terminate = (error: Error) => {
        failure ??= error;
        child.kill("SIGKILL");
      };
      const abort = () => terminate(new WorkbenchError("Request cancelled", 499));
      const deadline = setTimeout(
        () => terminate(new WorkbenchError("Native work exceeded the job limit", 504)),
        120_000,
      );
      deadline.unref();
      signal?.addEventListener("abort", abort, { once: true });
      child.stdout.on("data", (chunk: Buffer) => {
        bytes += chunk.length;
        if (bytes > 64 * 1024 * 1024)
          terminate(new WorkbenchError("Native report exceeded the output limit", 502));
        else stdout.push(chunk);
      });
      child.stderr.setEncoding("utf8");
      child.stderr.on("data", (text: string) => {
        stderr = (stderr + text).slice(-16_384);
      });
      child.on("error", (error) => {
        failure = new WorkbenchError(
          `Native map report could not start. Run bun run build:map-workbench. ${error.message}`,
          503,
        );
      });
      child.on("close", (code) => {
        clearTimeout(deadline);
        signal?.removeEventListener("abort", abort);
        if (failure) return reject(failure);
        if (code !== 0)
          return reject(new WorkbenchError(stderr.trim() || "Native map report failed", 502));
        try {
          complete(JSON.parse(Buffer.concat(stdout).toString("utf8")));
        } catch {
          reject(new WorkbenchError("Native map report returned invalid JSON", 502));
        }
      });
      child.stdin.on("error", () => {});
      child.stdin.end(JSON.stringify(request));
    });
}
