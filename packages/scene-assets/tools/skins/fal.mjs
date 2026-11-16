// The transport for paid generation: the installed `bento` CLI, which owns the
// login and the workspace, so no vendor key ever enters this process.
//
// - fal through Bento's service proxy: upload an input (`/storage/upload/
//   initiate`, then a PUT to the signed URL), submit, poll, fetch the result
//   and download its files. Every call is retried on a transient failure (a
//   502/503/504, a reset connection, a timeout), up to `retries` times, except
//   the paid submit, which is retried only when it surely never landed (a
//   502/503, a refused connection): a request is never paid twice.
// - Input images through Bento's model gateway (`bento local model chat`),
//   one attempt: a failed drawing is logged and redrawn by hand.
//
// `exec(args)` runs `bento` and resolves {code, stdout, stderr}; `fetch` reaches
// the signed vendor URLs. Tests inject both.
import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const TRANSIENT = /\b50[234]\b|ECONNRESET|ETIMEDOUT|EAI_AGAIN|EPIPE|socket hang up|timed? ?out/i;
// A paid submit is repeated only when it surely never reached the service: a
// 504, a reset or a timeout may come after it was accepted (and charged).
const REFUSED = /\b50[23]\b|ECONNREFUSED|EAI_AGAIN/i;
const TERMINAL = new Set(["COMPLETED", "FAILED", "ERROR", "CANCELLED"]);

export const runBento = (args, { timeout = 120_000 } = {}) =>
  new Promise((done) => {
    execFile("bento", args, { timeout, maxBuffer: 64 << 20 }, (error, stdout, stderr) => {
      const code = error ? (error.killed ? null : (error.code ?? 1)) : 0;
      done({ code, stdout, stderr: error?.killed ? `timed out after ${timeout} ms` : stderr });
    });
  });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const transient = ({ code, stdout, stderr }) =>
  code === 6 || code === null || TRANSIENT.test(`${stderr}\n${stdout}`);
const refused = ({ stdout, stderr }) => REFUSED.test(`${stderr}\n${stdout}`);

/** fal's queue paths are built from an endpoint's first two segments:
 * `tripo3d/h3.1/image-to-3d` polls `tripo3d/h3.1/requests/<id>`. */
export const requestPath = (endpoint, id) =>
  `/${endpoint.split("/").slice(0, 2).join("/")}/requests/${id}`;

export function fal({ exec = runBento, fetch = globalThis.fetch, wait = sleep, retries = 3 } = {}) {
  const retry = async (attempt) => {
    for (let n = 0; ; n++) {
      try {
        return await attempt();
      } catch (error) {
        if (!error.transient || n >= retries) throw error;
        await wait(2000 * 2 ** n);
      }
    }
  };
  // One `bento service proxy` call: the body in a file, the reply read back
  // from `--out`, and the response headers (printed on stdout) kept for the
  // charge Bento reports.
  const proxy = (method, path, { body, query, paid = false } = {}) =>
    retry(async () => {
      const dir = await mkdtemp(join(tmpdir(), "skin-generate-"));
      try {
        const out = join(dir, "out.json");
        const args = ["service", "proxy", method, "--service-id", "fal", "--path", path];
        if (query) args.push("--query", JSON.stringify(query));
        if (body !== undefined) {
          await writeFile(join(dir, "body.json"), JSON.stringify(body));
          args.push("--body-file", join(dir, "body.json"));
        }
        const run = await exec([...args, "--out", out]);
        if (run.code !== 0) {
          // The service's own reason, when it sent one, says why it refused.
          const reason = await readFile(out, "utf8").catch(() => "");
          const error = new Error(
            `bento ${method} ${path}: ${run.stderr.trim() || `exit ${run.code}`}${reason ? ` ${reason.trim().slice(0, 500)}` : ""}`,
          );
          error.transient = paid ? refused(run) : transient(run);
          throw error;
        }
        const charge = /x-duet-charge-usd"?\s*[:=]\s*"?([\d.]+)/i.exec(run.stdout);
        return {
          reply: JSON.parse(await readFile(out, "utf8")),
          charge: charge ? Number(charge[1]) : null,
        };
      } finally {
        await rm(dir, { recursive: true, force: true });
      }
    });
  const vendor = (method, url, init = {}) =>
    retry(async () => {
      let res;
      try {
        res = await fetch(url, { ...init, method });
      } catch (cause) {
        throw Object.assign(new Error(`${method} ${url}: ${cause.message}`), { transient: true });
      }
      if (!res.ok)
        throw Object.assign(new Error(`${method} ${url}: ${res.status}`), {
          transient: res.status >= 500,
        });
      return res;
    });

  return {
    /** Upload a local file to fal's CDN; its public URL. */
    async upload(path, contentType = "image/png") {
      const name = path.split("/").at(-1);
      const { reply } = await proxy("post", "/storage/upload/initiate", {
        query: { storage_type: "fal-cdn-v3" },
        body: { file_name: name, content_type: contentType },
      });
      await vendor("PUT", reply.upload_url, {
        headers: { "Content-Type": contentType },
        body: await readFile(path),
      });
      return reply.file_url;
    },
    /** Queue `body` on `endpoint`: {request_id, charge} (charge as Bento reports it). */
    async submit(endpoint, body) {
      const { reply, charge } = await proxy("post", `/${endpoint}`, { body, paid: true });
      if (!reply.request_id)
        throw new Error(`${endpoint} returned no request id: ${JSON.stringify(reply)}`);
      return { request_id: reply.request_id, charge };
    },
    /** Poll until the request ends or `deadline` ms pass; the last status
     * reply, or null when it is still running. */
    async poll(endpoint, id, { every = 15_000, deadline = 40 * 60_000, onStatus = () => {} } = {}) {
      const until = Date.now() + deadline;
      for (let last; ; ) {
        const { reply } = await proxy("get", `${requestPath(endpoint, id)}/status`);
        if (reply.status !== last) onStatus((last = reply.status));
        if (TERMINAL.has(reply.status)) return reply;
        if (Date.now() > until) return null;
        await wait(every);
      }
    },
    result: async (endpoint, id) => (await proxy("get", requestPath(endpoint, id))).reply,
    async download(url, dest) {
      await writeFile(dest, Buffer.from(await (await vendor("GET", url)).arrayBuffer()));
    },
  };
}

/** Draw one image through Bento's model gateway into `out`, one attempt. */
export async function draw(exec, model, size, prompt, out) {
  const run = await exec(
    [
      "local",
      "model",
      "chat",
      "--model",
      model,
      "--type",
      "image",
      "--size",
      size,
      "--max-retries",
      "0",
      "--out",
      out,
      prompt,
    ],
    { timeout: 300_000 },
  );
  if (run.code !== 0)
    throw new Error(`bento local model chat: ${run.stderr.trim() || `exit ${run.code}`}`);
}
