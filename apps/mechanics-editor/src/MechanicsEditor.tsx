import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  MECHANICS_API,
  type Json,
  type JsonObject,
  type MechanicsDraft,
  type MechanicsPreview,
  type MechanicsSnapshot,
} from "./protocol";
import {
  changeKey,
  draftEntry,
  editDraft,
  fieldOrigin,
  resolvedEntries,
  resetDraft,
  unitWeapons,
  weaponUsers,
  type EditTarget,
} from "./draft";
import {
  CONVERSION_CAPTIONS,
  displayFieldValue,
  entryFields,
  gameplayField,
  isObject,
  parseFieldValue,
  validateGameplayValue,
  valueAt,
  type GameplayField,
} from "./fields";
import "./mechanics.css";

interface TextEdit {
  text: string;
  error?: string;
}
interface EditorState {
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
const EditorContext = createContext<EditorState | null>(null);
const useEditor = () => {
  const editor = useContext(EditorContext);
  if (!editor) throw new Error("Mechanics controls need their editor.");
  return editor;
};
const rowName = (id: string, entry: JsonObject) =>
  typeof entry.name === "string" ? entry.name : id;
const emptyDraft = (snapshot: MechanicsSnapshot): MechanicsDraft => ({
  revision: snapshot.revision,
  changes: [],
});
const rawText = (value: Json | undefined) =>
  value === undefined || value === null
    ? ""
    : Array.isArray(value)
      ? value.join(", ")
      : typeof value === "object"
        ? JSON.stringify(value, null, 2)
        : String(value);

function textIsWithin(key: string, target: EditTarget, path: readonly string[]): boolean {
  const [section, id, unit, candidatePath] = JSON.parse(key.slice(0, key.lastIndexOf(":"))) as [
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
}

class RequestError extends Error {
  constructor(
    message: string,
    readonly conflict: boolean,
  ) {
    super(message);
  }
}
async function request<T>(suffix = "", draft?: MechanicsDraft): Promise<T> {
  const response = await fetch(
    `${MECHANICS_API}${suffix}`,
    draft
      ? {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(draft),
        }
      : undefined,
  );
  const body = (await response.json()) as T & { error?: string };
  if (!response.ok)
    throw new RequestError(
      body.error ?? `Request failed (${response.status}).`,
      response.status === 409,
    );
  return body;
}

/** The editable mount object omits parser defaults forbidden on that body type. */
function mountValue(target: EditTarget, value: Json | undefined): Json | undefined {
  if (!Array.isArray(value)) return value;
  return value.map((row) =>
    !isObject(row)
      ? row
      : Object.fromEntries(
          Object.entries(row).filter(
            ([key]) =>
              key === "name" ||
              gameplayField(target.section, ["mounts", String(row.name), key]) !== null,
          ),
        ),
  );
}

function referenceChoices(field: GameplayField, editor: EditorState, entry: JsonObject): string[] {
  if (!field.references) return [];
  if (field.references === "mounts")
    return Array.isArray(entry.mounts)
      ? entry.mounts.flatMap((m) => (isObject(m) && typeof m.name === "string" ? [m.name] : []))
      : [];
  const rows = editor.snapshot.catalog[field.references];
  if (isObject(rows)) return Object.keys(rows);
  if (Array.isArray(rows))
    return rows.flatMap((row) => (isObject(row) && typeof row.id === "string" ? [row.id] : []));
  if (field.references === "soldiers" || field.references === "weapons")
    return Object.keys(resolvedEntries(editor.snapshot.catalog, field.references));
  return [];
}

function ValueInput({
  target,
  field,
  entry,
  label,
  raw = false,
}: {
  target: EditTarget;
  field: GameplayField;
  entry: JsonObject;
  label: string;
  raw?: boolean;
}) {
  const editor = useEditor();
  const key = `${changeKey(target, field.path)}:${raw ? "raw" : "human"}`;
  const original = valueAt(entry, field.path);
  const value = field.kind === "mounts" ? mountValue(target, original) : original;
  const buffer = editor.texts[key];
  const text = buffer?.text ?? (raw ? rawText(value) : displayFieldValue(field, value, entry));
  const id = `me-${encodeURIComponent(key)}-${encodeURIComponent(label)}`;
  const emitText = (text: string) => {
    const parsed = raw
      ? parseFieldValue({ ...field, conversion: undefined }, text, entry)
      : parseFieldValue(field, text, entry);
    editor.setText(key, { text, error: parsed.error });
    if (parsed.error === undefined && parsed.value !== undefined)
      editor.change(target, field, parsed.value, entry, key);
  };
  const blur = () => {
    if (!buffer?.error) editor.setText(key, undefined);
  };
  const common = {
    id,
    "aria-label": label,
    "aria-invalid": Boolean(buffer?.error),
    "aria-describedby": buffer?.error ? `${id}-error` : undefined,
    disabled: editor.busy,
    value: text,
    onBlur: blur,
  };
  let input: ReactNode;
  if (field.kind === "boolean")
    input = (
      <select {...common} onChange={(event) => emitText(event.target.value)}>
        <option value="false">Off</option>
        <option value="true">On</option>
      </select>
    );
  else if (field.kind === "enum" || field.kind === "reference") {
    const choices = [
      ...new Set([
        ...(field.choices ?? referenceChoices(field, editor, entry)),
        ...(typeof value === "string" ? [value] : []),
      ]),
    ];
    input = (
      <select {...common} onChange={(event) => emitText(event.target.value)}>
        {field.optional && <option value="">None</option>}
        {choices.map((choice) => (
          <option key={choice} value={choice}>
            {choice}
          </option>
        ))}
      </select>
    );
  } else if (field.kind === "object") {
    input = (
      <select
        {...common}
        value={isObject(value) ? "enabled" : ""}
        onChange={(event) => {
          editor.setText(key, undefined);
          editor.change(target, field, event.target.value ? (field.initial ?? {}) : null, entry);
        }}
      >
        <option value="">None</option>
        <option value="enabled">Enabled</option>
      </select>
    );
  } else if (field.kind === "mounts")
    input = (
      <textarea
        {...common}
        rows={4}
        spellCheck={false}
        onChange={(event) => emitText(event.target.value)}
      />
    );
  else
    input = (
      <input
        {...common}
        type="text"
        inputMode={field.kind === "number" || field.kind === "ammo" ? "decimal" : "text"}
        spellCheck={false}
        placeholder={field.optional ? "None" : undefined}
        list={field.kind === "ammo" ? "mechanics-ammo" : undefined}
        onChange={(event) => emitText(event.target.value)}
      />
    );
  return (
    <div className={`me-input ${buffer?.error ? "me-invalid" : ""}`}>
      {input}
      {buffer?.error && (
        <span id={`${id}-error`} className="me-field-error" role="alert">
          {buffer.error}
        </span>
      )}
    </div>
  );
}

function FieldRow({
  target,
  field,
  entry,
  compactLabel,
}: {
  target: EditTarget;
  field: GameplayField;
  entry: JsonObject;
  compactLabel?: string;
}) {
  const editor = useEditor();
  const origin = fieldOrigin(editor.snapshot, target, field.path);
  const pending = editor.draft.changes.find(
    (c) => changeKey(c, c.path) === changeKey(target, field.path),
  );
  if (compactLabel)
    return <ValueInput target={target} field={field} entry={entry} label={compactLabel} />;
  const title = `${field.label}${field.unit ? ` · ${field.unit}` : ""}`;
  return (
    <div className={`me-field ${pending ? "me-field-changed" : ""}`}>
      <div className="me-field-copy">
        <label>{title}</label>
        <p>{field.explanation}</p>
        <div className="me-origin">
          <span>
            {pending?.restore
              ? "Restore queued"
              : pending
                ? "Local override queued"
                : origin.inherited
                  ? `Inherited from ${origin.id}`
                  : origin.local
                    ? `Authored on ${origin.id}`
                    : "Resolved default"}
          </span>
          <code>{origin.path}</code>
          {(origin.canRestore || pending) && !pending?.restore && (
            <button
              type="button"
              className="me-text-button"
              disabled={editor.busy}
              onClick={() => editor.restore(target, field, entry)}
            >
              {origin.canRestore || origin.inherited
                ? origin.restoresDefault
                  ? "Restore default value"
                  : "Restore inherited value"
                : "Undo edit"}
            </button>
          )}
        </div>
      </div>
      <div className="me-field-controls">
        <div>
          <span className="me-control-caption">
            {field.conversion ? "Battlefield value" : "Value"}
          </span>
          <ValueInput
            target={target}
            field={field}
            entry={entry}
            label={`${target.id} ${field.label}`}
          />
        </div>
        {field.conversion && (
          <div>
            <span className="me-control-caption">{CONVERSION_CAPTIONS[field.conversion]}</span>
            <ValueInput
              target={target}
              field={field}
              entry={entry}
              raw
              label={`${target.id} ${field.label} — ${CONVERSION_CAPTIONS[field.conversion]}`}
            />
          </div>
        )}
      </div>
    </div>
  );
}

function EntryForm({ target, original }: { target: EditTarget; original: JsonObject }) {
  const editor = useEditor();
  const entry = draftEntry(original, editor.draft, target, editor.snapshot);
  const groups = new Map<string, GameplayField[]>();
  for (const field of entryFields(target.section, entry)) {
    const group =
      field.path[0] === "mounts" && field.path.length > 1
        ? `Mount · ${field.path[1]}`
        : field.group;
    groups.set(group, [...(groups.get(group) ?? []), field]);
  }
  return (
    <div className="me-groups">
      {[...groups].map(([group, fields]) => (
        <section className="me-group" key={group}>
          <h4>{group}</h4>
          {fields.map((field) => (
            <FieldRow key={field.path.join("/")} target={target} field={field} entry={entry} />
          ))}
        </section>
      ))}
    </div>
  );
}

function ExpandedUnit({ id, unit }: { id: string; unit: JsonObject }) {
  const editor = useEditor();
  const effective = draftEntry(unit, editor.draft, { section: "units", id }, editor.snapshot);
  const slots = valueAt(effective, ["body", "squad", "slots"]);
  const soldierKinds = Array.isArray(slots)
    ? [...new Set(slots.filter((v): v is string => typeof v === "string"))]
    : [];
  const soldiers = resolvedEntries(editor.snapshot.catalog, "soldiers");
  const weapons = resolvedEntries(editor.snapshot.catalog, "weapons");
  return (
    <div className="me-expanded">
      <div className="me-expanded-heading">
        <div>
          <span className="me-eyebrow">Exact unit · {id}</span>
          <h3>{rowName(id, unit)}</h3>
          <p>{String(unit.description ?? "")}</p>
        </div>
        <p className="me-scope">
          Overrides are authored on this type. Descendants inherit them.
          <br />
          Linked weapon changes are global.
        </p>
      </div>
      <EntryForm target={{ section: "units", id }} original={unit} />
      {soldierKinds.length > 0 && (
        <section className="me-linked">
          <h3>Soldiers in this unit</h3>
          <p>
            Health and carried weapons are defined by soldier kind. Edits create a variant for this
            type’s slots; descendants inherit those slots.
          </p>
          {soldierKinds.map(
            (kind) =>
              soldiers[kind] && (
                <details className="me-linked-entry" key={kind}>
                  <summary>
                    {rowName(kind, soldiers[kind])}
                    <span>
                      {Array.isArray(slots) ? slots.filter((v) => v === kind).length : 0} slots ·{" "}
                      {kind}
                    </span>
                  </summary>
                  <EntryForm
                    target={{ section: "soldiers", id: kind, unit: id }}
                    original={soldiers[kind]}
                  />
                </details>
              ),
          )}
        </section>
      )}
      <section className="me-linked">
        <h3>Shared weapons</h3>
        <p>
          Each row is edited globally. Every affected exact unit is listed here and checked again in
          preview.
        </p>
        {unitWeapons(editor.snapshot, id, editor.draft).map(
          (weapon) =>
            weapons[weapon] && (
              <details className="me-linked-entry" key={weapon}>
                <summary>
                  {rowName(weapon, weapons[weapon])}
                  <span>{weapon} · global</span>
                </summary>
                <div className="me-impact">
                  Used by{" "}
                  {weaponUsers(editor.snapshot, weapon, editor.draft)
                    .map((user) =>
                      rowName(user, resolvedEntries(editor.snapshot.catalog, "units")[user]),
                    )
                    .join(", ")}
                </div>
                <EntryForm target={{ section: "weapons", id: weapon }} original={weapons[weapon]} />
              </details>
            ),
        )}
      </section>
    </div>
  );
}

export default function MechanicsEditor() {
  const [snapshot, setSnapshot] = useState<MechanicsSnapshot | null>(null);
  const [draft, setDraft] = useState<MechanicsDraft | null>(null);
  const [texts, setTexts] = useState<Record<string, TextEdit>>({});
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [conflict, setConflict] = useState(false);
  const [reloadArmed, setReloadArmed] = useState(false);
  const [saved, setSaved] = useState(false);
  const [preview, setPreview] = useState<{ value: MechanicsPreview; draft: string } | null>(null);
  const errors = Object.values(texts).some((t) => t.error);
  const dirty = Boolean(draft?.changes.length || Object.keys(texts).length);
  const draftJSON = JSON.stringify(draft);
  useEffect(() => {
    let active = true;
    request<MechanicsSnapshot>()
      .then((value) => {
        if (active) {
          setSnapshot(value);
          setDraft(emptyDraft(value));
        }
      })
      .catch((error) => {
        if (active) setError(String(error.message));
      });
    return () => {
      active = false;
    };
  }, []);
  useEffect(() => {
    if (!dirty) return;
    const leaving = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", leaving);
    return () => window.removeEventListener("beforeunload", leaving);
  }, [dirty]);
  const units = useMemo(
    () => (snapshot ? resolvedEntries(snapshot.catalog, "units") : {}),
    [snapshot],
  );
  const weapons = useMemo(
    () => (snapshot ? resolvedEntries(snapshot.catalog, "weapons") : {}),
    [snapshot],
  );
  const filtered = Object.entries(units).filter(([id, unit]) => {
    const weapons = snapshot && draft ? unitWeapons(snapshot, id, draft) : [];
    return [
      id,
      unit.name,
      unit.faction,
      unit.family,
      ...(Array.isArray(unit.roles) ? unit.roles : []),
      ...weapons,
    ]
      .join(" ")
      .toLowerCase()
      .includes(search.toLowerCase().trim());
  });
  const handleFailure = (failure: unknown) => {
    setError(failure instanceof Error ? failure.message : String(failure));
    if (failure instanceof RequestError && failure.conflict) setConflict(true);
  };
  const reload = async () => {
    if (dirty && !reloadArmed) {
      setReloadArmed(true);
      return;
    }
    setBusy(true);
    try {
      const value = await request<MechanicsSnapshot>();
      setSnapshot(value);
      setDraft(emptyDraft(value));
      setTexts({});
      setPreview(null);
      setConflict(false);
      setError("");
      setSaved(false);
      setReloadArmed(false);
    } catch (failure) {
      handleFailure(failure);
    } finally {
      setBusy(false);
    }
  };
  const previewChanges = async () => {
    if (!draft || errors) return;
    setBusy(true);
    setError("");
    setSaved(false);
    try {
      setPreview({ value: await request<MechanicsPreview>("/preview", draft), draft: draftJSON });
    } catch (failure) {
      setPreview(null);
      handleFailure(failure);
    } finally {
      setBusy(false);
    }
  };
  const save = async () => {
    if (!draft || errors || !preview || preview.draft !== draftJSON) return;
    setBusy(true);
    setError("");
    try {
      const value = await request<MechanicsSnapshot>("/save", draft);
      setSnapshot(value);
      setDraft(emptyDraft(value));
      setTexts({});
      setPreview(null);
      setConflict(false);
      setSaved(true);
    } catch (failure) {
      handleFailure(failure);
    } finally {
      setBusy(false);
    }
  };
  const applyDraft = (
    next: MechanicsDraft,
    target: EditTarget,
    field: GameplayField,
    entry: JsonObject,
    textKey?: string,
  ) => {
    if (!snapshot) return;
    const slots = valueAt(entry, ["body", "squad", "slots"]);
    const nextSlots = valueAt(draftEntry(entry, next, target, snapshot), [
      "body",
      "squad",
      "slots",
    ]);
    const removedSoldiers = new Set<string>();
    if (target.section === "units" && Array.isArray(slots))
      for (const kind of slots)
        if (typeof kind === "string" && (!Array.isArray(nextSlots) || !nextSlots.includes(kind)))
          removedSoldiers.add(kind);
    const removedTarget = (candidate: EditTarget) =>
      candidate.section === "soldiers" &&
      candidate.unit === target.id &&
      removedSoldiers.has(candidate.id);
    setDraft({ ...next, changes: next.changes.filter((change) => !removedTarget(change)) });
    setPreview(null);
    setSaved(false);
    setReloadArmed(false);
    setTexts((current) =>
      Object.fromEntries(
        Object.entries(current).filter(
          ([key, edit]) =>
            !(textIsWithin(key, target, field.path) && key !== textKey) &&
            ![...removedSoldiers].some((id) =>
              textIsWithin(key, { section: "soldiers", id, unit: target.id }, []),
            ) &&
            !(
              target.section === "weapons" &&
              field.path[0] === "range_m" &&
              !edit.error &&
              key.startsWith(`${changeKey(target, ["scatter_mrad"])}:`)
            ),
        ),
      ),
    );
  };
  const editor: EditorState | null =
    snapshot && draft
      ? {
          snapshot,
          draft,
          texts,
          busy,
          setText: (key, edit) =>
            setTexts((current) => {
              const next = { ...current };
              if (edit) next[key] = edit;
              else delete next[key];
              return next;
            }),
          change: (target, field, value, entry, textKey) => {
            if (validateGameplayValue(target.section, field.path, value)) return;
            applyDraft(
              editDraft(draft, target, field.path, value, entry),
              target,
              field,
              entry,
              textKey,
            );
          },
          restore: (target, field, entry) =>
            applyDraft(
              resetDraft(draft, target, field.path, entry, snapshot),
              target,
              field,
              entry,
            ),
        }
      : null;
  return (
    <main className="mechanics-editor">
      <header className="me-header">
        <div>
          <a className="me-back" href="/">
            ← Developer menu
          </a>
          <div className="me-title">
            <h1>Mechanics</h1>
            <span>LOCAL AUTHORING</span>
          </div>
          <p>Exact units. Shared weapons. The rules behind the battle.</p>
        </div>
        <div className="me-status">
          <span className={dirty ? "me-unsaved" : ""}>
            {dirty ? `${draft?.changes.length ?? 0} pending changes` : "Authored fixtures"}
          </span>
          <span>Applies to new or restarted battles</span>
        </div>
      </header>
      <div className="me-toolbar">
        <label className="me-search">
          <span>Find unit</span>
          <input
            type="search"
            aria-label="Search units"
            placeholder="Name, id, family, role or weapon…"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </label>
        <span className="me-count">
          {filtered.length} / {Object.keys(units).length} units
        </span>
        <div className="me-actions">
          <button type="button" disabled={busy || !snapshot} onClick={reload}>
            Reload sources
          </button>
          <button
            type="button"
            disabled={busy || !draft?.changes.length || errors || conflict}
            onClick={previewChanges}
          >
            {busy ? "Working…" : "Preview changes"}
          </button>
          <button
            type="button"
            className="me-primary"
            disabled={busy || errors || conflict || !preview || preview.draft !== draftJSON}
            onClick={save}
          >
            Save authored JSON
          </button>
        </div>
      </div>
      {error && (
        <div className="me-error" role="alert">
          <strong>
            {conflict ? "Sources changed outside this editor" : "Changes could not be accepted"}
          </strong>
          <p>{error}</p>
          {conflict && (
            <p>Your draft is preserved. Reloading discards it and reads the current files.</p>
          )}
        </div>
      )}
      {reloadArmed && (
        <div className="me-reload" role="alert">
          <p>Reload will discard your pending changes and unfinished text.</p>
          <button type="button" disabled={busy} onClick={reload}>
            Discard draft and reload
          </button>
          <button type="button" onClick={() => setReloadArmed(false)}>
            Keep editing
          </button>
        </div>
      )}
      {saved && (
        <div className="me-success" role="status">
          Saved authored JSON and synchronized the catalog. Running battles continue; new or
          restarted battles use these rules.
        </div>
      )}
      {!editor ? (
        <p className="me-loading">
          {error ? "The local mechanics API is unavailable." : "Reading authored rules…"}
        </p>
      ) : (
        <EditorContext.Provider value={editor}>
          <datalist id="mechanics-ammo">
            <option value="unlimited" />
          </datalist>
          <p className="me-scroll-hint">Scroll sideways to see all unit values →</p>
          <div className="me-sheet-scroll">
            <table className="me-sheet">
              <thead>
                <tr>
                  <th>Unit / exact type</th>
                  <th>
                    Cost <span>points</span>
                  </th>
                  <th>
                    Off-road <span>km/h</span>
                  </th>
                  <th>
                    Road <span>km/h</span>
                  </th>
                  <th>
                    Sight <span>m</span>
                  </th>
                  <th>
                    Health <span>HP / soldiers</span>
                  </th>
                  <th>Weapons</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map(([id, original]) => {
                  const entry = draftEntry(
                    original,
                    editor.draft,
                    { section: "units", id },
                    editor.snapshot,
                  );
                  const name = rowName(id, original);
                  const mobility = isObject(entry.mobility) ? Object.keys(entry.mobility)[0] : "";
                  const common = (path: string[], label: string) => {
                    const f = gameplayField("units", path);
                    return f ? (
                      <FieldRow
                        target={{ section: "units", id }}
                        field={f}
                        entry={entry}
                        compactLabel={`${name} ${label}`}
                      />
                    ) : (
                      "—"
                    );
                  };
                  const slots = valueAt(entry, ["body", "squad", "slots"]);
                  return (
                    <UnitRows
                      key={id}
                      expanded={expanded === id}
                      row={
                        <tr className={expanded === id ? "me-selected" : ""}>
                          <td>
                            <button
                              type="button"
                              className="me-unit-name"
                              aria-label={`${expanded === id ? "Collapse" : "Expand"} ${name}`}
                              aria-expanded={expanded === id}
                              onClick={() => setExpanded(expanded === id ? null : id)}
                            >
                              <span className="me-expander">{expanded === id ? "−" : "+"}</span>
                              <span>
                                <strong>{name}</strong>
                                <small>
                                  {id} · {String(original.family ?? "")}
                                </small>
                              </span>
                            </button>
                          </td>
                          <td>{common(["cost"], "deployment cost")}</td>
                          <td>{common(["mobility", mobility, "offroad_kmh"], "off-road speed")}</td>
                          <td>{common(["mobility", mobility, "road_kmh"], "road speed")}</td>
                          <td>{common(["sensors", "ground_m"], "sight range")}</td>
                          <td>
                            {Array.isArray(slots) ? (
                              <span className="me-health-summary">{slots.length} soldiers</span>
                            ) : (
                              common(["body", "hull", "hp"], "hull health")
                            )}
                          </td>
                          <td className="me-weapons-cell">
                            {unitWeapons(editor.snapshot, id, editor.draft).map((weapon) => (
                              <span key={weapon}>{String(weapons[weapon]?.name ?? weapon)}</span>
                            ))}
                          </td>
                        </tr>
                      }
                      detail={<ExpandedUnit id={id} unit={original} />}
                    />
                  );
                })}
              </tbody>
            </table>
            {filtered.length === 0 && (
              <p className="me-no-results">
                No unit matches “{search}”. Your pending edits are retained.
              </p>
            )}
          </div>
          {preview && (
            <section className="me-preview" aria-label="Save preview">
              <div className="me-preview-heading">
                <div>
                  <span className="me-eyebrow">Validated publication</span>
                  <h2>Preview authored changes</h2>
                </div>
                <span>
                  {preview.value.files.length} files · {preview.value.affectedUnits.length} affected{" "}
                  {preview.value.affectedUnits.length === 1 ? "unit" : "units"}
                </span>
              </div>
              <p>These exact formatted replacements will be saved. Running battles continue.</p>
              <div className="me-impact">
                <strong>Affected units</strong>{" "}
                {preview.value.affectedUnits
                  .map((id) => (units[id] ? `${rowName(id, units[id])} (${id})` : id))
                  .join(", ") || "None"}
              </div>
              {preview.value.warnings.map((warning, index) => (
                <p className="me-warning" key={index}>
                  {warning}
                </p>
              ))}
              <ul className="me-change-list">
                {editor.draft.changes.map((change) => (
                  <li key={changeKey(change, change.path)}>
                    <code>
                      {change.section}.{change.id}.{change.path.join(".")}
                    </code>
                    <span>
                      {change.restore ? "Restore inheritance" : JSON.stringify(change.value)}
                      {change.unit
                        ? ` · override on ${change.unit}`
                        : change.section === "weapons"
                          ? " · shared globally"
                          : ""}
                    </span>
                  </li>
                ))}
              </ul>
              {preview.value.files.map((file) => (
                <details className="me-file" key={file.path}>
                  <summary>
                    {file.path}
                    <span>Exact JSON</span>
                  </summary>
                  <div className="me-file-columns">
                    <div>
                      <h4>Before</h4>
                      <pre>{file.before}</pre>
                    </div>
                    <div>
                      <h4>After · saved bytes</h4>
                      <pre>{file.after}</pre>
                    </div>
                  </div>
                </details>
              ))}
            </section>
          )}
        </EditorContext.Provider>
      )}
    </main>
  );
}

function UnitRows({
  row,
  detail,
  expanded,
}: {
  row: ReactNode;
  detail: ReactNode;
  expanded: boolean;
}) {
  return (
    <>
      {row}
      {expanded && (
        <tr className="me-detail-row">
          <td colSpan={7}>{detail}</td>
        </tr>
      )}
    </>
  );
}
