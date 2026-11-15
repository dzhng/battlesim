// Adapt a seat's wire protocol to the existing page/authority channel.
// No simulation controls cross this seam: only orders and previews go up.
import type { SeatLink } from "../../../../packages/multiplayer/src/link";
import type {
  PreviewAnswer,
  PreviewQuery,
  StreamOpen,
  UpBatch,
  UpReply,
} from "../../../../packages/multiplayer/src/wire";
import {
  readJson,
  resumingReader,
  StreamBroken,
} from "../../../../packages/multiplayer/src/stream/index";
import type { SimConnection } from "../sim/client";
import type { SimReply, SimRequest, SideName } from "../sim/protocol";

/** A lost reply must not block later orders; a retry keeps the batch's identity. */
const BATCH_RETRY_MS = 2000;
type Preview = Extract<
  SimRequest,
  { type: "purchase_preview" | "move_preview" | "building_preview" }
>;

export function remoteConnection(link: SeatLink): SimConnection {
  return (receive, fail) => {
    let closed = false;
    let initialized = false;
    /** The battle has been described to the page (`ready`). */
    let described = false;
    let started = false;
    let side: SideName;
    let description: StreamOpen | null = null;
    // A batch's number only has to be later than every earlier one from this
    // seat, a reloaded page's included, so it counts up from the clock.
    let seq = Date.now();
    const orders: UpBatch["items"] = [];
    let flight: { batch: UpBatch; timer: ReturnType<typeof setTimeout> | null } | null = null;
    let preview: Preview | null = null;
    let activePreview: Preview | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
    let wakeReconnect: (() => void) | undefined;

    const close = () => {
      if (closed) return;
      closed = true;
      if (flight?.timer) clearTimeout(flight.timer);
      clearTimeout(reconnectTimer);
      wakeReconnect?.();
      orders.length = 0;
      preview = null;
      link.close();
    };
    const failed = (error: unknown) => {
      if (closed) return;
      close();
      fail(error instanceof Error ? error.message : String(error));
    };
    const refused = (request: "command" | "preview", id: number, reason: string) =>
      receive({ type: "request_refused", request, id, reason });

    function sendOrders() {
      if (closed || flight || !orders.length) return;
      const batch = { seq: ++seq, items: orders.splice(0) };
      flight = { batch, timer: null };
      attempt(batch);
    }
    function attempt(batch: UpBatch) {
      if (closed || flight?.batch !== batch) return;
      flight.timer = setTimeout(() => attempt(batch), BATCH_RETRY_MS);
      // A late attempt may settle first. Only the first answer for this
      // batch is consumed; every later one is silent.
      void link.up(batch).then(
        (reply: UpReply) => {
          if (closed || flight?.batch !== batch) return;
          if (reply.seq !== batch.seq)
            return failed(new Error("Order reply has the wrong batch sequence"));
          if (flight.timer) clearTimeout(flight.timer);
          flight = null;
          if ("refused" in reply)
            for (const item of batch.items) refused("command", item.id, reply.refused);
          else for (const result of reply.results) receive({ type: "ack", ...result });
          sendOrders();
        },
        () => {
          /* The retry timer, not a failed request, controls retry cadence. */
        },
      );
    }

    const dropPreview = (request: Preview, reason: string) =>
      refused("preview", request.id, reason);
    function sendPreview() {
      if (closed || activePreview || !preview) return;
      const request = preview;
      preview = null;
      activePreview = request;
      const query: PreviewQuery =
        request.type === "purchase_preview"
          ? { kind: "purchase", variant: request.variant, destination: request.destination }
          : request.type === "move_preview"
            ? { kind: "move", move: request.move }
            : { kind: "building", building: request.building };
      void link
        .preview(query)
        .then(
          (answer: PreviewAnswer) => {
            if (closed) return;
            if ("dropped" in answer) dropPreview(request, "dropped");
            else if (request.type === "purchase_preview" && "purchase" in answer)
              receive({ type: "purchase_preview", id: request.id, placement: answer.purchase });
            else if (request.type === "move_preview" && "moves" in answer)
              receive({ type: "move_preview", id: request.id, destinations: answer.moves });
            else if (request.type === "building_preview" && "building" in answer)
              receive({ type: "building_preview", id: request.id, placement: answer.building });
            else throw new Error("Preview answer does not match its query");
          },
          () => {
            if (!closed) dropPreview(request, "unavailable");
          },
        )
        .catch(failed)
        .finally(() => {
          activePreview = null;
          sendPreview();
        });
    }

    /** The seat's stream as one endless sequence of records. */
    const records = () =>
      resumingReader(
        (after) => {
          if (closed) throw new StreamBroken("connection closed");
          return link.records(after);
        },
        {
          sleep: (ms) =>
            new Promise<void>((resolve) => {
              if (closed) return resolve();
              wakeReconnect = resolve;
              reconnectTimer = setTimeout(() => {
                wakeReconnect = undefined;
                resolve();
              }, ms);
            }),
        },
      );
    const tickOf = (payload: Uint8Array) =>
      new DataView(payload.buffer, payload.byteOffset, payload.byteLength).getUint32(0, true);

    /** Learn the battle's layout, rate and tick, then let the stream go. A
     *  page may take as long as it likes to load: until it starts, nothing
     *  of the battle is read or kept. */
    async function describe() {
      for await (const record of records()) {
        if (closed) return;
        if (record.kind === "open") description = readJson<StreamOpen>(record);
        else if (record.kind === "end")
          throw new Error(`Battle stream ended: ${readJson<{ reason: string }>(record).reason}`);
        else if (record.kind === "publication") {
          if (!description?.layout || !description.tickHz)
            throw new Error("Battle stream has no layout or tick rate");
          receive({
            type: "ready",
            layout: description.layout,
            tickHz: description.tickHz,
            tick: tickOf(record.payload),
          });
          return;
        }
      }
    }

    /** Follow the battle from a snapshot taken now. */
    async function follow() {
      for await (const record of records()) {
        if (closed) return;
        if (record.kind === "open") {
          // A reader that joins a running battle missed its status record.
          const { status } = readJson<StreamOpen>(record);
          if (status) receive({ type: "status", ...status });
        } else if (record.kind === "publication") {
          const buffer = record.payload.slice(24).buffer;
          receive({
            type: "publication",
            side,
            tick: tickOf(record.payload),
            digest: new TextDecoder().decode(record.payload.subarray(4, 20)),
            stepMs: new DataView(
              record.payload.buffer,
              record.payload.byteOffset,
              record.payload.byteLength,
            ).getFloat32(20, true),
            buffer,
            length: buffer.byteLength / 4,
          });
        } else if (record.kind === "status") {
          const status = readJson<Extract<SimReply, { type: "status" }>>(record);
          receive({ type: "status", status: status.status, slow: status.slow });
        } else if (record.kind === "end") {
          const { reason } = readJson<{ reason: string }>(record);
          if (reason !== "finished") throw new Error(`Battle stream ended: ${reason}`);
          for (const item of [...(flight?.batch.items ?? []), ...orders])
            refused("command", item.id, "ended");
          if (activePreview) dropPreview(activePreview, "ended");
          if (preview) dropPreview(preview, "ended");
          receive({ type: "status", status: "finished", slow: false });
          close();
          return;
        }
      }
    }
    return {
      send(request) {
        if (closed) {
          if (request.type === "command") refused("command", request.id, "ended");
          else if (
            request.type === "purchase_preview" ||
            request.type === "move_preview" ||
            request.type === "building_preview"
          )
            dropPreview(request, "ended");
          return;
        }
        switch (request.type) {
          case "connect":
            if (initialized) return;
            initialized = true;
            side = request.side;
            void describe()
              .then(() => {
                described = true;
                if (started) return follow();
              })
              .catch(failed);
            return;
          case "start":
            if (started) return;
            started = true;
            if (described) void follow().catch(failed);
            return;
          case "command":
            orders.push({ id: request.id, order: request.order, queued: request.queued });
            queueMicrotask(sendOrders);
            return;
          case "purchase_preview":
          case "move_preview":
          case "building_preview":
            if (preview) dropPreview(preview, "superseded");
            preview = request;
            queueMicrotask(sendPreview);
            return;
          case "credit":
            return;
          case "dispose":
            return close();
          default:
            return failed(new Error(`Request ${request.type} is unavailable online`));
        }
      },
      close,
    };
  };
}
