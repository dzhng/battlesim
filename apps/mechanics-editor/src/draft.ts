import { gameplayField, isObject, valueAt } from "./fields";
import {
  targetKey,
  type Json,
  type JsonObject,
  type MechanicsChange,
  type MechanicsDraft,
  type EditableSources,
  type Section,
} from "./protocol";

export interface EditTarget {
  section: Section;
  id: string;
  unit?: string;
}
export const changeKey = (target: EditTarget, path: readonly string[]) =>
  JSON.stringify([target.section, target.id, target.unit ?? null, path]);
const sameTarget = (a: EditTarget, b: EditTarget) => targetKey(a) === targetKey(b);
const samePath = (a: readonly string[], b: readonly string[]) =>
  a.length === b.length && a.every((p, i) => p === b[i]);
const prefix = (a: readonly string[], b: readonly string[]) =>
  a.length <= b.length && a.every((p, i) => p === b[i]);

function replaceChange(draft: MechanicsDraft, change: MechanicsChange): MechanicsDraft {
  return {
    ...draft,
    changes: [
      ...draft.changes.filter(
        (c) =>
          !sameTarget(c, change) ||
          (!samePath(c.path, change.path) && !prefix(change.path, c.path)),
      ),
      change,
    ],
  };
}

/** Canonical authored changes only; no inheritance resolution is reproduced here. */
export function editDraft(
  draft: MechanicsDraft,
  target: EditTarget,
  path: string[],
  value: Json,
  entry: JsonObject,
): MechanicsDraft {
  let next = replaceChange(draft, { ...target, path, value });
  if (
    target.section === "weapons" &&
    samePath(path, ["range_m"]) &&
    typeof value === "number" &&
    value > 0 &&
    typeof entry.range_m === "number" &&
    typeof entry.scatter_mrad === "number"
  ) {
    next = replaceChange(next, {
      ...target,
      path: ["scatter_mrad"],
      value: (entry.scatter_mrad * entry.range_m) / value,
    });
  }
  return next;
}

export function restoreDraft(
  draft: MechanicsDraft,
  target: EditTarget,
  path: string[],
): MechanicsDraft {
  return replaceChange(draft, { ...target, path, restore: true });
}

/** Restoring or undoing range obeys the same landing-spread contract as editing it. */
export function resetDraft(
  draft: MechanicsDraft,
  target: EditTarget,
  path: string[],
  entry: JsonObject,
  snapshot: EditableSources,
): MechanicsDraft {
  const origin = fieldOrigin(snapshot, target, path);
  const restore = origin.canRestore || origin.inherited;
  const original = resolvedEntries(snapshot.catalog, target.section)[target.id];
  const value = restore ? origin.parentValue : valueAt(original, path);
  let next = draft;
  if (target.section === "weapons" && samePath(path, ["range_m"]) && typeof value === "number") {
    next = editDraft(next, target, path, value, entry);
    if (draftEntry(original, next, target).scatter_mrad === original.scatter_mrad)
      next = {
        ...next,
        changes: next.changes.filter(
          (c) => !sameTarget(c, target) || !samePath(c.path, ["scatter_mrad"]),
        ),
      };
  }
  return restore
    ? restoreDraft(next, target, path)
    : {
        ...next,
        changes: next.changes.filter((c) => !sameTarget(c, target) || !prefix(path, c.path)),
      };
}

function setAt(object: JsonObject, path: readonly string[], value: Json): void {
  let cursor: JsonObject = object;
  for (let i = 0; i < path.length - 1; i++) {
    const key = path[i];
    const existing = cursor[key];
    if (Array.isArray(existing)) {
      const id = path[++i];
      let row = existing.find((v) => isObject(v) && v.id === id);
      if (!isObject(row)) {
        row = { id };
        existing.push(row);
      }
      cursor = row;
    } else {
      if (!isObject(existing)) cursor[key] = {};
      cursor = cursor[key] as JsonObject;
    }
  }
  cursor[path.at(-1)!] = value;
}

export function draftEntry(
  original: JsonObject,
  draft: MechanicsDraft,
  target: EditTarget,
  snapshot?: EditableSources,
): JsonObject {
  const entry = structuredClone(original);
  for (const change of draft.changes.filter((c) => sameTarget(c, target))) {
    if (change.restore) {
      const parent = snapshot && fieldOrigin(snapshot, target, change.path).parentValue;
      // Show known inherited values, including native defaults.
      // Admission owns values absent from the resolved parent and part patches.
      if (parent !== undefined) setAt(entry, change.path, structuredClone(parent));
    } else if (change.value !== undefined) setAt(entry, change.path, structuredClone(change.value));
  }
  return entry;
}

export function resolvedEntries(catalog: JsonObject, section: Section): Record<string, JsonObject> {
  if (section === "weapons" && isObject(catalog.weapons)) return objectEntries(catalog.weapons);
  const documents = catalog.documents;
  if (Array.isArray(documents)) {
    const combined: Record<string, JsonObject> = {};
    for (const doc of documents)
      if (isObject(doc) && isObject(doc[section]))
        Object.assign(combined, objectEntries(doc[section]));
    return combined;
  }
  const rows = catalog[section];
  if (Array.isArray(rows))
    return Object.fromEntries(
      rows.flatMap((row) => (isObject(row) && typeof row.id === "string" ? [[row.id, row]] : [])),
    );
  return isObject(rows) ? objectEntries(rows) : {};
}
const objectEntries = (value: JsonObject): Record<string, JsonObject> =>
  Object.fromEntries(
    Object.entries(value).filter((row): row is [string, JsonObject] => isObject(row[1])),
  );

export interface FieldOrigin {
  id: string;
  path: string;
  inherited: boolean;
  local: boolean;
  canRestore: boolean;
  restoresDefault: boolean;
  parentValue?: Json;
}

/** Source provenance is a leaf lookup along authored parents, not a second merger. */
export function fieldOrigin(
  snapshot: EditableSources,
  target: EditTarget,
  path: readonly string[],
): FieldOrigin {
  const source = (id: string) =>
    snapshot.documents.flatMap((doc) => {
      const row = valueAt(doc.value, [target.section, id]);
      return isObject(row) ? [{ row, file: doc.path }] : [];
    })[0];
  const first = source(target.id);
  let owner = first;
  let ownerId = target.id;
  let parentValue: Json | undefined;
  const chain = new Set<string>();
  const local = first ? valueAt(first.row, path) !== undefined : false;
  let current = first;
  while (current && typeof current.row.extends === "string" && !chain.has(current.row.extends)) {
    const id = current.row.extends;
    chain.add(id);
    current = source(id);
    if (current && valueAt(current.row, path) !== undefined) {
      if (parentValue === undefined) parentValue = valueAt(current.row, path);
      if (!local && owner === first) {
        owner = current;
        ownerId = id;
      }
    }
  }
  const descriptor = gameplayField(target.section, path);
  const authoredParent = parentValue;
  const parentId = first?.row.extends;
  const resolvedParent =
    typeof parentId === "string"
      ? resolvedEntries(snapshot.catalog, target.section)[parentId]
      : undefined;
  if (resolvedParent) parentValue = valueAt(resolvedParent, path);
  // Native rows include parser defaults and inherited mounts (by id). Abstract
  // parents may be absent; optional leaves can still be removed for admission.
  const restoresDefault =
    authoredParent === undefined && (parentValue !== undefined || Boolean(descriptor?.optional));
  return {
    id: ownerId,
    path: owner?.file ?? "Resolved default",
    inherited: !local && ownerId !== target.id,
    local,
    canRestore:
      local &&
      (parentValue !== undefined || Boolean(descriptor?.optional)) &&
      Boolean(first && typeof first.row.extends === "string"),
    restoresDefault,
    parentValue,
  };
}

export function unitWeapons(
  snapshot: EditableSources,
  id: string,
  draft: MechanicsDraft,
): string[] {
  const units = resolvedEntries(snapshot.catalog, "units");
  const original = units[id];
  if (!original) return [];
  const unit = draftEntry(original, draft, { section: "units", id }, snapshot);
  const kinds = valueAt(unit, ["body", "squad", "slots"]);
  const soldiers = resolvedEntries(snapshot.catalog, "soldiers");
  const mounts = Array.isArray(kinds)
    ? kinds.flatMap((kind) =>
        typeof kind === "string" && soldiers[kind]
          ? ((draftEntry(
              soldiers[kind],
              draft,
              { section: "soldiers", id: kind, unit: id },
              snapshot,
            ).mounts as Json[]) ?? [])
          : [],
      )
    : Array.isArray(unit.mounts)
      ? unit.mounts
      : [];
  return [
    ...new Set(
      mounts.flatMap((m) =>
        isObject(m) && Array.isArray(m.weapons)
          ? m.weapons.filter((v): v is string => typeof v === "string")
          : [],
      ),
    ),
  ];
}

export function weaponUsers(
  snapshot: EditableSources,
  weapon: string,
  draft: MechanicsDraft,
): string[] {
  return Object.keys(resolvedEntries(snapshot.catalog, "units")).filter((id) =>
    unitWeapons(snapshot, id, draft).includes(weapon),
  );
}
