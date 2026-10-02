// Sample only this Chromium instance's processes. Summed RSS may double-count
// shared pages; physical footprint is the kernel's charged-memory measure.
import { execFileSync } from "node:child_process";

// SDK offsetof(ri_instructions) is 248 bytes (slot 31); cycles is slot 32.
const READ = `import ctypes,json,sys
lib=ctypes.CDLL('/usr/lib/libproc.dylib')
rows=[]
for text in sys.argv[1:]:
 pid=int(text); info=(ctypes.c_uint64*64)()
 if lib.proc_pid_rusage(pid,4,ctypes.byref(info))==0:
  rows.append({'pid':pid,'instructions':info[31],'rss':info[8],'footprint':info[9],'lifetimePeak':info[30]})
print(json.dumps(rows))`;

/** Instruction deltas are a lower bound: a process can exit between samples. */
export async function startupResources(page) {
  if (process.platform !== "darwin") return { finish: async () => null };
  const cdp = await page.context().browser().newBrowserCDPSession();
  const counters = new Map();
  let peakRSS = 0,
    peakFootprint = 0,
    footprintHighWater = 0;
  let first = true;
  const stages = {};
  const sample = async () => {
    const { processInfo } = await cdp.send("SystemInfo.getProcessInfo");
    const rows = JSON.parse(
      execFileSync("python3", ["-c", READ, ...processInfo.map((p) => String(p.id))], {
        encoding: "utf8",
      }),
    );
    for (const row of rows) {
      const old = counters.get(row.pid);
      counters.set(row.pid, {
        first: old?.first ?? (first ? row.instructions : 0),
        last: row.instructions,
      });
    }
    first = false;
    const stage = await page
      .evaluate(
        () => document.querySelector("[data-testid=loading-stage]")?.textContent ?? "playable",
      )
      .catch(() => "navigation");
    const rss = rows.reduce((n, r) => n + r.rss, 0);
    const footprint = rows.reduce((n, r) => n + r.footprint, 0);
    footprintHighWater = Math.max(
      footprintHighWater,
      rows.reduce((n, r) => n + r.lifetimePeak, 0),
    );
    const old = stages[stage];
    stages[stage] = {
      rss: Math.max(old?.rss ?? 0, rss),
      footprint: Math.max(old?.footprint ?? 0, footprint),
    };
    peakRSS = Math.max(
      peakRSS,
      rows.reduce((n, r) => n + r.rss, 0),
    );
    peakFootprint = Math.max(
      peakFootprint,
      rows.reduce((n, r) => n + r.footprint, 0),
    );
  };
  await sample();
  let sampling = Promise.resolve();
  const timer = setInterval(() => {
    sampling = sampling.then(sample);
  }, 500);
  return {
    async finish() {
      clearInterval(timer);
      try {
        await sampling;
        await sample();
        return {
          peakRSS,
          peakFootprint,
          footprintHighWater,
          instructions: [...counters.values()].reduce((n, r) => n + r.last - r.first, 0),
          samplingMs: 500,
          stages,
        };
      } finally {
        await cdp.detach();
      }
    },
  };
}
