import type { SoundCatalog } from "../../../packages/battle-audio/src/catalog";
import type { LookSound, Review, Snapshot } from "./protocol";

export type SoundOperation = {
  id: number;
  kind: "load" | "reload" | "preview" | "save";
  draftJSON?: string;
};

export type SoundOutcome =
  | { kind: "error"; message: string }
  | { kind: "saved" }
  | { kind: "reloaded" };

export interface SoundReviewCertificate {
  value: Review;
  draftJSON: string;
}

export interface SoundWorkbenchState {
  snapshot: Snapshot | null;
  draft: SoundCatalog | null;
  lookSounds: Record<string, LookSound>;
  review: SoundReviewCertificate | null;
  operation: SoundOperation | null;
  outcome: SoundOutcome | null;
}

export const initialSoundWorkbenchState: SoundWorkbenchState = {
  snapshot: null,
  draft: null,
  lookSounds: {},
  review: null,
  operation: null,
  outcome: null,
};

type SoundAction =
  | { type: "operation-started"; operation: SoundOperation }
  | { type: "snapshot-received"; id: number; snapshot: Snapshot }
  | { type: "preview-received"; id: number; value: Review }
  | { type: "operation-failed"; id: number; message: string }
  | { type: "outcome-cleared" }
  | { type: "review-closed" }
  | { type: "catalog-edited"; draft: SoundCatalog }
  | { type: "look-sounds-edited"; lookSounds: Record<string, LookSound> };

const editable = (snapshot: Snapshot) => ({
  draft: structuredClone(snapshot.catalog),
  lookSounds: structuredClone(
    Object.fromEntries(Object.entries(snapshot.looks).map(([name, look]) => [name, look.sound])),
  ),
});

const currentDraftJSON = (state: SoundWorkbenchState) =>
  JSON.stringify({
    revision: state.snapshot?.revision,
    catalog: state.draft,
    lookSounds: state.lookSounds,
  });

function acceptedSnapshot(
  state: SoundWorkbenchState,
  snapshot: Snapshot,
  operation: SoundOperation,
): SoundWorkbenchState {
  return {
    ...state,
    snapshot,
    ...editable(snapshot),
    review: null,
    operation: null,
    outcome:
      operation.kind === "save"
        ? { kind: "saved" }
        : operation.kind === "reload"
          ? { kind: "reloaded" }
          : null,
  };
}

export function soundWorkbenchReducer(
  state: SoundWorkbenchState,
  action: SoundAction,
): SoundWorkbenchState {
  switch (action.type) {
    case "operation-started":
      return {
        ...state,
        operation: action.operation,
        outcome: null,
      };
    case "snapshot-received": {
      const operation = state.operation;
      if (
        !operation ||
        operation.id !== action.id ||
        !["load", "reload", "save"].includes(operation.kind)
      )
        return state;
      return acceptedSnapshot(state, action.snapshot, operation);
    }
    case "preview-received": {
      const operation = state.operation;
      if (
        !operation ||
        operation.id !== action.id ||
        operation.kind !== "preview" ||
        currentDraftJSON(state) !== operation.draftJSON
      )
        return operation?.id === action.id ? { ...state, operation: null } : state;
      return {
        ...state,
        operation: null,
        review: { value: action.value, draftJSON: operation.draftJSON! },
        outcome: null,
      };
    }
    case "operation-failed":
      if (state.operation?.id !== action.id) return state;
      return {
        ...state,
        operation: null,
        outcome: { kind: "error", message: action.message },
      };
    case "outcome-cleared":
      return { ...state, outcome: null };
    case "review-closed":
      return { ...state, review: null };
    case "catalog-edited":
      return {
        ...state,
        draft: structuredClone(action.draft),
        review: null,
        outcome: null,
      };
    case "look-sounds-edited":
      return {
        ...state,
        lookSounds: structuredClone(action.lookSounds),
        review: null,
        outcome: null,
      };
  }
}

export const soundWorkbenchBusy = (state: SoundWorkbenchState) =>
  Boolean(state.operation && state.operation.kind !== "load");

export const soundWorkbenchDirty = (state: SoundWorkbenchState) =>
  Boolean(
    state.snapshot &&
    state.draft &&
    (JSON.stringify(state.snapshot.catalog) !== JSON.stringify(state.draft) ||
      Object.entries(state.snapshot.looks).some(
        ([name, look]) => JSON.stringify(look.sound) !== JSON.stringify(state.lookSounds[name]),
      )),
  );

export const soundWorkbenchCanSave = (state: SoundWorkbenchState) =>
  Boolean(
    state.review &&
    state.draft &&
    state.review.draftJSON === currentDraftJSON(state) &&
    !soundWorkbenchBusy(state),
  );
