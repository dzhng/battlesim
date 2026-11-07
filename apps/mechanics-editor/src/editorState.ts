import type { MechanicsDraft, MechanicsPreview, MechanicsSnapshot } from "./protocol";
import type { EditTarget } from "./draft";

export interface TextEdit {
  text: string;
  error?: string;
}

export type EditorOperation = {
  id: number;
  kind: "load" | "reload" | "preview" | "save";
  draftJSON?: string;
};

export type EditorOutcome = { kind: "error"; message: string } | { kind: "saved" };

export interface ReviewCertificate {
  value: MechanicsPreview;
  draftJSON: string;
}

export interface MechanicsEditorState {
  snapshot: MechanicsSnapshot | null;
  draft: MechanicsDraft | null;
  texts: Record<string, TextEdit>;
  review: ReviewCertificate | null;
  operation: EditorOperation | null;
  outcome: EditorOutcome | null;
  conflict: boolean;
  reloadArmed: boolean;
}

export const initialEditorState: MechanicsEditorState = {
  snapshot: null,
  draft: null,
  texts: {},
  review: null,
  operation: null,
  outcome: null,
  conflict: false,
  reloadArmed: false,
};

type EditorAction =
  | { type: "operation-started"; operation: EditorOperation }
  | { type: "snapshot-received"; id: number; snapshot: MechanicsSnapshot }
  | { type: "preview-received"; id: number; value: MechanicsPreview }
  | { type: "operation-failed"; id: number; message: string; conflict: boolean }
  | { type: "text-changed"; key: string; edit: TextEdit }
  | { type: "text-cleared"; key: string }
  | {
      type: "draft-edited";
      draft: MechanicsDraft;
      target: EditTarget;
      path: readonly string[];
      textKey?: string;
      removedSoldiers: readonly string[];
    }
  | { type: "reload-armed" }
  | { type: "reload-disarmed" };

const emptyDraft = (snapshot: MechanicsSnapshot): MechanicsDraft => ({
  revision: snapshot.revision,
  changes: [],
});

function textIsWithin(key: string, target: EditTarget, path: readonly string[]): boolean {
  const separator = key.lastIndexOf(":");
  if (separator < 0) return false;
  try {
    const [section, id, unit, candidatePath] = JSON.parse(key.slice(0, separator)) as [
      EditTarget["section"],
      string,
      string | null,
      string[],
    ];
    return (
      section === target.section &&
      id === target.id &&
      unit === (target.unit ?? null) &&
      path.every((part, index) => candidatePath[index] === part)
    );
  } catch {
    return false;
  }
}

function acceptedSnapshot(
  state: MechanicsEditorState,
  snapshot: MechanicsSnapshot,
): MechanicsEditorState {
  const saved = state.operation?.kind === "save";
  const reload = state.operation?.kind === "reload";
  return {
    ...state,
    snapshot,
    draft: emptyDraft(snapshot),
    texts: {},
    review: null,
    operation: null,
    outcome: saved ? { kind: "saved" } : null,
    conflict: false,
    reloadArmed: reload ? false : state.reloadArmed,
  };
}

export function editorReducer(
  state: MechanicsEditorState,
  action: EditorAction,
): MechanicsEditorState {
  switch (action.type) {
    case "operation-started":
      return {
        ...state,
        operation: action.operation,
        outcome:
          action.operation.kind === "preview" || action.operation.kind === "save"
            ? null
            : state.outcome,
      };
    case "snapshot-received":
      if (
        state.operation?.id !== action.id ||
        !["load", "reload", "save"].includes(state.operation.kind)
      )
        return state;
      return acceptedSnapshot(state, action.snapshot);
    case "preview-received":
      if (state.operation?.id !== action.id) return state;
      if (
        state.operation.kind !== "preview" ||
        currentDraftJSON(state) !== state.operation.draftJSON
      )
        return { ...state, operation: null };
      return {
        ...state,
        operation: null,
        review: { value: action.value, draftJSON: state.operation.draftJSON! },
        outcome: null,
      };
    case "operation-failed":
      if (state.operation?.id !== action.id) return state;
      return {
        ...state,
        operation: null,
        review: state.operation.kind === "preview" ? null : state.review,
        outcome: { kind: "error", message: action.message },
        conflict: state.conflict || action.conflict,
      };
    case "text-changed":
      return {
        ...state,
        texts: { ...state.texts, [action.key]: action.edit },
      };
    case "text-cleared": {
      const texts = { ...state.texts };
      delete texts[action.key];
      return { ...state, texts };
    }
    case "draft-edited": {
      const texts = Object.fromEntries(
        Object.entries(state.texts).filter(
          ([key]) =>
            !(textIsWithin(key, action.target, action.path) && key !== action.textKey) &&
            !action.removedSoldiers.some((id) =>
              textIsWithin(key, { section: "soldiers", id, unit: action.target.id }, []),
            ),
        ),
      );
      return {
        ...state,
        draft: action.draft,
        texts,
        review: null,
        outcome: state.outcome?.kind === "saved" ? null : state.outcome,
        reloadArmed: false,
      };
    }
    case "reload-armed":
      return { ...state, reloadArmed: true };
    case "reload-disarmed":
      return { ...state, reloadArmed: false };
  }
}

export const editorErrors = (state: MechanicsEditorState) =>
  Object.values(state.texts).some((text) => text.error);

export const editorDirty = (state: MechanicsEditorState) =>
  Boolean(state.draft?.changes.length || Object.keys(state.texts).length);

export const editorBusy = (state: MechanicsEditorState) =>
  Boolean(state.operation && state.operation.kind !== "load");

export const currentDraftJSON = (state: MechanicsEditorState) => JSON.stringify(state.draft);

export const canPreview = (state: MechanicsEditorState) =>
  Boolean(state.draft?.changes.length) && !editorErrors(state) && !state.conflict;

export const canSave = (state: MechanicsEditorState) =>
  Boolean(
    state.draft &&
    state.review &&
    state.review.draftJSON === currentDraftJSON(state) &&
    !editorErrors(state) &&
    !state.conflict,
  );

export const needsReloadConfirmation = (state: MechanicsEditorState) =>
  editorDirty(state) && !state.reloadArmed;

export type { EditorAction };
