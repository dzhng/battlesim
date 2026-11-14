import { Link } from "react-router";
import { useEffect, useMemo, useReducer, useRef, useState, type ReactNode } from "react";
import { menuPage } from "@apps/battle-lab/src/menu/pages";
import { isGameDocument } from "../../../web/src/battle/catalog/compose";
import { request, RequestError } from "./api";
import { DerivedFacts } from "./DerivedFacts";
import { EditorContext, useEditor, type EditorState } from "./editorContext";
import {
  targetKey,
  type Json,
  type JsonObject,
  type MechanicsChange,
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
  fieldValue,
  entryFields,
  gameplayField,
  isObject,
  parseFieldValue,
  rawText,
  validateGameplayValue,
  enabledValue,
  valueAt,
  type GameplayField,
} from "./fields";
import {
  canPreview,
  canSave,
  editorBusy,
  editorDirty,
  editorReducer,
  initialEditorState,
  needsReloadConfirmation,
} from "./editorState";
import "./mechanics.css";

const rowName = (id: string, entry: JsonObject) =>
  typeof entry.name === "string" ? entry.name : id;
function ChangeSummary({
  change,
  preview,
}: {
  change: MechanicsChange;
  preview: MechanicsPreview;
}) {
  const editor = useEditor();
  const before = resolvedEntries(editor.snapshot.catalog, change.section)[change.id];
  const field = gameplayField(change.section, change.path);
  const id = change.unit ? preview.soldierIds[targetKey(change)] : change.id;
  const after = resolvedEntries(preview.catalog, change.section)[id];
  const beforeValue = valueAt(before, change.path);
  const afterValue = valueAt(after, change.path);
  const formatted = (value: Json | undefined) => {
    value = fieldValue(field, value);
    if (value === undefined) return "Unset";
    if (value === null) return "Disabled";
    const text = field ? displayFieldValue(field, value) : rawText(value);
    return `${text}${field?.unit ? ` ${field.unit}` : ""}`;
  };
  return (
    <li>
      <div>
        <strong className="me-change-label">{field?.label ?? change.path.join(".")}</strong>
        <code>
          {change.section}.{change.id}.{change.path.join(".")}
        </code>
      </div>
      <div className="me-change-values">
        <strong>
          {formatted(beforeValue)} → {formatted(afterValue)}
        </strong>
        {field?.conversion && (
          <small>
            {CONVERSION_CAPTIONS[field.conversion]}: {rawText(beforeValue)} → {rawText(afterValue)}
          </small>
        )}
        <small>
          {change.restore ? "Restore inheritance · " : ""}
          {change.unit
            ? `Override on ${change.unit}`
            : change.section === "weapons"
              ? "Shared globally"
              : "Type override"}
        </small>
      </div>
    </li>
  );
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
              key === "id" ||
              key === "name" ||
              gameplayField(target.section, ["mounts", String(row.id), key]) !== null,
          ),
        ),
  );
}

function referenceChoices(field: GameplayField, editor: EditorState, entry: JsonObject): string[] {
  if (!field.references) return [];
  if (field.references === "mounts")
    return Array.isArray(entry.mounts)
      ? entry.mounts.flatMap((m) => (isObject(m) && typeof m.id === "string" ? [m.id] : []))
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
  const original = fieldValue(field, valueAt(entry, field.path));
  const value = field.kind === "mounts" ? mountValue(target, original) : original;
  const buffer = editor.texts[key];
  const text = buffer?.text ?? (raw ? rawText(value) : displayFieldValue(field, value));
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
          const row =
            target.section === "weapons"
              ? resolvedEntries(editor.snapshot.catalog, target.section)[target.id]
              : undefined;
          editor.change(target, field, event.target.value ? enabledValue(field, row) : null, entry);
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
                <DerivedFacts row={weapons[weapon]} />
                <EntryForm target={{ section: "weapons", id: weapon }} original={weapons[weapon]} />
              </details>
            ),
        )}
      </section>
    </div>
  );
}

export default function MechanicsEditor() {
  const [state, dispatch] = useReducer(editorReducer, initialEditorState);
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState<string | null>(null);
  const lastEdit = useRef<HTMLElement | null>(null);
  const [hasLastEdit, setHasLastEdit] = useState(false);
  const nextOperation = useRef(0);
  const { snapshot, draft, texts, review: preview, outcome, reloadArmed } = state;
  const busy = editorBusy(state);
  const dirty = editorDirty(state);
  const error = outcome?.kind === "error" ? outcome.message : "";
  const conflict = state.conflict;
  const saved = outcome?.kind === "saved";

  useEffect(() => {
    if (lastEdit.current && !lastEdit.current.isConnected) {
      lastEdit.current = null;
      setHasLastEdit(false);
    }
  });

  const run = async <T,>(
    kind: "load" | "reload" | "preview" | "save",
    suffix: string,
    body?: MechanicsDraft,
  ) => {
    const id = ++nextOperation.current;
    dispatch({
      type: "operation-started",
      operation: { id, kind, draftJSON: body ? JSON.stringify(body) : undefined },
    });
    try {
      const value = await request<T>(suffix, body);
      return { id, value };
    } catch (failure) {
      dispatch({
        type: "operation-failed",
        id,
        message: failure instanceof Error ? failure.message : String(failure),
        conflict: failure instanceof RequestError && failure.conflict,
      });
      return null;
    }
  };

  useEffect(() => {
    let active = true;
    void run<MechanicsSnapshot>("load", "").then((result) => {
      if (active && result)
        dispatch({ type: "snapshot-received", id: result.id, snapshot: result.value });
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
  // Test units (`fixtures/units/test`) are fakes tests and labs run on, never
  // game content: listed after the roster, under their own heading.
  const testUnits = useMemo(
    () =>
      new Set(
        (snapshot?.documents ?? [])
          .filter((document) => !isGameDocument(document.path))
          .flatMap((document) =>
            Object.keys(isObject(document.value.units) ? document.value.units : {}),
          ),
      ),
    [snapshot],
  );
  const matching = Object.entries(units).filter(([id, unit]) => {
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
  const filtered = [
    ...matching.filter(([id]) => !testUnits.has(id)),
    ...matching.filter(([id]) => testUnits.has(id)),
  ];
  const firstTestUnit = filtered.find(([id]) => testUnits.has(id))?.[0];
  const reload = async () => {
    if (!snapshot) return;
    if (needsReloadConfirmation(state)) {
      dispatch({ type: "reload-armed" });
      return;
    }
    const result = await run<MechanicsSnapshot>("reload", "");
    if (result) dispatch({ type: "snapshot-received", id: result.id, snapshot: result.value });
  };
  const previewChanges = async () => {
    if (!canPreview(state) || !draft) return;
    const captured = draft;
    const result = await run<MechanicsPreview>("preview", "/preview", captured);
    if (result)
      dispatch({
        type: "preview-received",
        id: result.id,
        value: result.value,
      });
  };
  const save = async () => {
    if (!canSave(state) || !draft) return;
    const captured = draft;
    const result = await run<MechanicsSnapshot>("save", "/save", captured);
    if (result) dispatch({ type: "snapshot-received", id: result.id, snapshot: result.value });
  };
  const applyDraft = (
    next: MechanicsDraft,
    target: EditTarget,
    field: GameplayField,
    entry: JsonObject,
    textKey?: string,
  ) => {
    if (!snapshot || !draft) return;
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
    dispatch({
      type: "draft-edited",
      draft: {
        ...next,
        changes: next.changes.filter(
          (change) =>
            !(
              change.section === "soldiers" &&
              change.unit === target.id &&
              removedSoldiers.has(change.id)
            ),
        ),
      },
      target,
      path: field.path,
      textKey,
      removedSoldiers: [...removedSoldiers],
    });
  };
  const editor: EditorState | null =
    snapshot && draft
      ? {
          snapshot,
          draft,
          texts,
          busy,
          setText: (key, edit) =>
            dispatch(edit ? { type: "text-changed", key, edit } : { type: "text-cleared", key }),
          change: (target, field, value, entry, textKey) => {
            if (validateGameplayValue(target.section, field.path, value)) return;
            applyDraft(editDraft(draft, target, field.path, value), target, field, entry, textKey);
          },
          restore: (target, field, entry) =>
            applyDraft(resetDraft(draft, target, field.path, snapshot), target, field, entry),
        }
      : null;
  return (
    <main
      className="mechanics-editor"
      onChangeCapture={(event) => {
        const input = event.target;
        if (
          input instanceof HTMLElement &&
          input.matches("input, textarea, select") &&
          input.closest(".me-input")
        ) {
          lastEdit.current = input;
          setHasLastEdit(true);
        }
      }}
    >
      <header className="me-header">
        <div>
          <Link className="me-back" to={menuPage("developer").path}>
            ← {menuPage("developer").title}
          </Link>
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
          <span>Reload the game page to apply saved rules</span>
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
          <button type="button" disabled={busy || !canPreview(state)} onClick={previewChanges}>
            {busy ? "Working…" : "Preview changes"}
          </button>
          <button
            type="button"
            className="me-primary"
            disabled={busy || !canSave(state)}
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
          {hasLastEdit && lastEdit.current?.isConnected && (
            <button
              type="button"
              onClick={() => {
                if (!lastEdit.current?.isConnected) return;
                let details = lastEdit.current.closest("details");
                while (details) {
                  details.open = true;
                  details = details.parentElement?.closest("details") ?? null;
                }
                lastEdit.current.scrollIntoView({ block: "center", behavior: "smooth" });
                lastEdit.current.focus({ preventScroll: true });
              }}
            >
              Back to last edit
            </button>
          )}
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
          <button type="button" onClick={() => dispatch({ type: "reload-disarmed" })}>
            Keep editing
          </button>
        </div>
      )}
      {saved && (
        <div className="me-success" role="status">
          Saved authored JSON and synchronized the catalog. Reload the game page to use these rules.
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
                      heading={
                        id === firstTestUnit
                          ? "Test units · fakes tests and labs run on, never in the game"
                          : undefined
                      }
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
                  <ChangeSummary
                    key={changeKey(change, change.path)}
                    change={change}
                    preview={preview.value}
                  />
                ))}
              </ul>
              {preview.value.files.map((file) => (
                <details className="me-file" key={file.path}>
                  <summary>
                    {file.path}
                    <span>Exact JSON</span>
                  </summary>
                  <p className="me-json-hint">
                    Scroll each JSON panel down and sideways to inspect the full replacement.
                  </p>
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
  heading,
  row,
  detail,
  expanded,
}: {
  heading?: string;
  row: ReactNode;
  detail: ReactNode;
  expanded: boolean;
}) {
  return (
    <>
      {heading && (
        <tr className="me-group">
          <th colSpan={7} scope="rowgroup">
            {heading}
          </th>
        </tr>
      )}
      {row}
      {expanded && (
        <tr className="me-detail-row">
          <td colSpan={7}>{detail}</td>
        </tr>
      )}
    </>
  );
}
