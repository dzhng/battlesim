// The editor as its controls read it: the snapshot and draft they edit, the
// texts typed so far, and the edits they may make.
import { createContext, useContext } from "react";
import type { EditTarget } from "./draft";
import type { TextEdit } from "./editorState";
import type { GameplayField } from "./fields";
import type { Json, JsonObject, MechanicsDraft, MechanicsSnapshot } from "./protocol";

export interface EditorState {
  snapshot: MechanicsSnapshot;
  draft: MechanicsDraft;
  texts: Record<string, TextEdit>;
  busy: boolean;
  setText: (key: string, text: TextEdit | undefined) => void;
  change: (
    target: EditTarget,
    field: GameplayField,
    value: Json,
    entry: JsonObject,
    textKey?: string,
  ) => void;
  restore: (target: EditTarget, field: GameplayField, entry: JsonObject) => void;
}

export const EditorContext = createContext<EditorState | null>(null);

export const useEditor = () => {
  const editor = useContext(EditorContext);
  if (!editor) throw new Error("Mechanics controls need their editor.");
  return editor;
};
