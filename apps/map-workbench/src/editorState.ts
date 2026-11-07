import type {
  Documents,
  Draft,
  Inspection,
  Report,
  RunRequest,
  SaveReview,
  Snapshot,
} from "./protocol";

export type DraftResult = { report: Report; request: RunRequest };
export type ArtifactSelection = { artifactId: string; id: string };
export type ArtifactInspection = Inspection & { artifactId: string };

export interface MapEditorState {
  snapshot: Snapshot | null;
  draft: Draft | null;
  loaded: Snapshot | null;
  raw: Record<string, string>;
  unfinished: Set<string>;
  latest: DraftResult | null;
  accepted: DraftResult | null;
  baseline: Report | null;
  view: "draft" | "saved";
  inspection: ArtifactInspection | null;
  selected: ArtifactSelection | null;
  review: SaveReview | null;
}

export const initialEditorState: MapEditorState = {
  snapshot: null,
  draft: null,
  loaded: null,
  raw: {},
  unfinished: new Set(),
  latest: null,
  accepted: null,
  baseline: null,
  view: "draft",
  inspection: null,
  selected: null,
  review: null,
};

export const sameValue = (left: unknown, right: unknown) =>
  JSON.stringify(left) === JSON.stringify(right);

export type EditorAction =
  | { type: "snapshot-accepted"; snapshot: Snapshot }
  | { type: "sources-loaded"; snapshot: Snapshot }
  | { type: "draft-edited"; documents: Documents; fieldId: string; text: string; valid: boolean }
  | { type: "field-undone"; documents: Documents; fieldId: string }
  | {
      type: "draft-reapplied";
      snapshot: Snapshot;
      documents: Documents;
      raw: Record<string, string>;
    }
  | { type: "choice-changed" }
  | { type: "generation-completed"; report: Report; request: RunRequest }
  | { type: "baseline-set"; report: Report }
  | { type: "view-set"; view: "draft" | "saved" }
  | { type: "inspection-completed"; artifactId: string; inspection: Inspection }
  | { type: "selection-set"; selection: ArtifactSelection | null }
  | { type: "review-set"; review: SaveReview | null };

function acceptedSnapshot(state: MapEditorState, snapshot: Snapshot): MapEditorState {
  return {
    ...state,
    snapshot,
    draft: { revision: snapshot.revision, documents: snapshot.documents },
    loaded: null,
    raw: {},
    unfinished: new Set(),
    baseline: null,
    view: "draft",
    review: null,
  };
}

export function editorReducer(state: MapEditorState, action: EditorAction): MapEditorState {
  switch (action.type) {
    case "snapshot-accepted":
      return acceptedSnapshot(state, action.snapshot);
    case "sources-loaded":
      return { ...state, loaded: action.snapshot };
    case "draft-edited": {
      const raw = { ...state.raw, [action.fieldId]: action.text };
      const wasUnfinished = state.unfinished.has(action.fieldId);
      const membershipChanged = action.valid === wasUnfinished;
      const unfinished = membershipChanged ? state.unfinished : new Set(state.unfinished);
      if (membershipChanged) {
        if (action.valid) unfinished.delete(action.fieldId);
        else unfinished.add(action.fieldId);
      }
      const documentsChanged = state.draft?.documents !== action.documents;
      if (!documentsChanged && !membershipChanged) return { ...state, raw };
      return {
        ...state,
        draft:
          documentsChanged && state.draft
            ? { ...state.draft, documents: action.documents }
            : state.draft,
        raw,
        unfinished,
        review: null,
      };
    }
    case "field-undone": {
      const raw = { ...state.raw };
      delete raw[action.fieldId];
      const unfinished = new Set(state.unfinished);
      unfinished.delete(action.fieldId);
      return {
        ...state,
        draft: state.draft ? { ...state.draft, documents: action.documents } : null,
        raw,
        unfinished,
        review: null,
      };
    }
    case "draft-reapplied":
      return {
        ...state,
        snapshot: action.snapshot,
        draft: { revision: action.snapshot.revision, documents: action.documents },
        loaded: null,
        raw: action.raw,
        view: "draft",
        baseline: null,
        review: null,
      };
    case "choice-changed":
      return {
        ...state,
        baseline: null,
        view: "draft",
        review: null,
      };
    case "generation-completed":
      return {
        ...state,
        latest: { report: action.report, request: action.request },
        accepted:
          action.report.status === "ok"
            ? { report: action.report, request: action.request }
            : state.accepted,
      };
    case "baseline-set":
      return { ...state, baseline: action.report, view: "saved" };
    case "view-set":
      return { ...state, view: action.view };
    case "inspection-completed":
      return { ...state, inspection: { artifactId: action.artifactId, ...action.inspection } };
    case "selection-set":
      return { ...state, selected: action.selection };
    case "review-set":
      return { ...state, review: action.review };
  }
}

export function isCurrent(state: MapEditorState, choice: RunRequest["choice"] | null): boolean {
  const latest = state.latest;
  return (
    state.unfinished.size === 0 &&
    !!latest &&
    !!state.draft &&
    !!choice &&
    latest.request.draft.documents === state.draft.documents &&
    sameValue(latest.report.choice, choice)
  );
}

export function displayedReport(state: MapEditorState): Report | undefined {
  return state.view === "saved" ? (state.baseline ?? undefined) : state.accepted?.report;
}

export function displayedSelection(state: MapEditorState): string | null {
  const shown = displayedReport(state);
  const selected = state.selected;
  if (!shown || !selected || selected.artifactId !== shown.artifactId) return null;
  return selected.id;
}

export function displayedInspection(state: MapEditorState): Inspection | Report | undefined {
  const shown = displayedReport(state);
  if (!shown) return undefined;
  const inspection = state.inspection;
  if (!inspection || inspection.artifactId !== shown.artifactId) return shown;
  return inspection;
}
