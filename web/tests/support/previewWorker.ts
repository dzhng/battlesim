// A preparation worker for lobby tests: it records the preview request it
// was sent, and the test answers it by hand, or it admits every request
// itself (`PreviewWorker.admitting`).
import { act } from "@testing-library/react";
import type { PrepareMessage, PrepareReply } from "../../src/battle/prepare/protocol";

export class PreviewWorker {
  static all: PreviewWorker[] = [];
  /** Admit each request just after it is posted, as a real worker would
   *  answer: on its own turn, outside any `act`. */
  static admitting = false;
  onmessage: ((event: MessageEvent<PrepareReply>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  message: (PrepareMessage & { type: "prepare" }) | null = null;
  terminated = false;
  constructor() {
    PreviewWorker.all.push(this);
  }
  postMessage(message: PrepareMessage) {
    if (message.type !== "prepare") return;
    this.message = message;
    if (PreviewWorker.admitting)
      setTimeout(() => this.onmessage?.({ data: this.admitted() } as MessageEvent<PrepareReply>));
  }
  terminate() {
    this.terminated = true;
  }
  get request() {
    return this.message!.request;
  }
  get seed() {
    return this.request.map_source.request.seed;
  }
  /** The map is admitted: the request comes back with its picture. */
  admit() {
    return this.reply(this.admitted());
  }
  private admitted(): PrepareReply {
    return {
      type: "prepared",
      battle: {
        scenario: "{}",
        report: { request: this.request } as never,
        preview: { map: "{}", sites: "{}", svg: `<svg data-seed="${this.seed}"></svg>` },
      },
    };
  }
  /** The map is refused, for a reason another seed can fix by default. */
  refuse(code = "generation_failed", stage: "map" | "request" = "map") {
    return this.reply({
      type: "refused",
      stage,
      diagnostics: [{ code, feature: null, location: "$", message: "no room" }],
    });
  }
  /** Deliver `reply`, and let the page settle on it. */
  private reply(reply: PrepareReply) {
    return act(async () => this.onmessage?.({ data: reply } as MessageEvent<PrepareReply>));
  }
}
