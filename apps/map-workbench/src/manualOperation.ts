import type { MapChoice } from "../../../web/src/maps/source";
import type { Draft, SaveReview, Snapshot } from "./protocol";

export type ManualKind =
  | "compare"
  | "inspect"
  | "sight"
  | "sample"
  | "reload"
  | "review"
  | "save"
  | "export";

export interface ManualCapture {
  draft?: Draft;
  snapshot?: Snapshot;
  choice?: MapChoice;
  artifactId?: string;
  fingerprint?: string;
  crop?: string;
  choices?: MapChoice[];
  discard?: boolean;
  review?: SaveReview;
}

export interface ManualOperation {
  kind: ManualKind;
  controller: AbortController;
  captured: ManualCapture;
  cancellationGeneration: number;
  completion: Promise<void>;
}

export function downloadJson(name: string, value: unknown) {
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(value, null, 2)], { type: "application/json" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  URL.revokeObjectURL(url);
}

interface ManualOwnerOptions {
  runner: { cancel(): Promise<void> };
  alive: () => boolean;
  currentInput: () => {
    draft: Draft | null;
    choice: MapChoice | null;
    latest: { request: { draft: Draft }; report: { choice: MapChoice } } | null;
    valid: boolean;
  };
  onKind: (kind: ManualKind | null) => void;
  onError: (error: string) => void;
  onStatus: (status: string) => void;
  request: (draft: Draft, choice: MapChoice) => void;
}

export function createManualOwner(options: ManualOwnerOptions) {
  let current: ManualOperation | null = null;
  let cancellationGeneration = 0;

  async function start(
    kind: ManualKind,
    captured: ManualCapture,
    action: (operation: ManualOperation, owned: () => boolean) => Promise<void>,
  ) {
    const previous = current;
    const record: ManualOperation = {
      kind,
      controller: new AbortController(),
      captured,
      cancellationGeneration,
      completion: Promise.resolve(),
    };
    current = record;
    options.onKind(record.kind);
    previous?.controller.abort();
    options.onError("");
    const owned = () =>
      options.alive() &&
      current === record &&
      !record.controller.signal.aborted &&
      record.cancellationGeneration === cancellationGeneration;
    const completion = (async () => {
      try {
        if (previous) await previous.completion;
        if (owned()) {
          await options.runner.cancel();
          if (owned()) await action(record, owned);
        }
      } catch (error) {
        if (owned()) options.onError(error instanceof Error ? error.message : String(error));
      } finally {
        const canResume = owned();
        if (current === record) {
          current = null;
          options.onKind(null);
          const now = options.currentInput();
          if (
            canResume &&
            now.draft &&
            now.choice &&
            now.valid &&
            (now.latest?.request.draft.documents !== now.draft.documents ||
              JSON.stringify(now.latest?.report.choice) !== JSON.stringify(now.choice))
          )
            options.request(now.draft, now.choice);
        }
      }
    })();
    record.completion = completion;
    await completion;
  }

  function cancel() {
    ++cancellationGeneration;
    current?.controller.abort();
    void options.runner.cancel();
    options.onStatus("Cancelled; previous result retained");
  }

  function cancelSample() {
    if (current?.kind === "sample") current.controller.abort();
  }

  return {
    start,
    cancel,
    cancelSample,
    abort: () => current?.controller.abort(),
  };
}
