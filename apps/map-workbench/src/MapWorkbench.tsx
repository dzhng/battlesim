import { useEffect, useMemo, useRef, useState } from "react";
import {
  canonicalSeed,
  MAP_SIZES,
  MAP_TYPES,
  newSeed,
  type MapChoice,
  type MapSize,
  type MapType,
} from "../../../web/src/maps/source";
import type {
  Documents,
  Draft,
  Field,
  Inspection,
  Report,
  RunRequest,
  SaveReview,
  SightReport,
  Snapshot,
  WorkbenchAPI,
} from "./protocol";
import { editField, fieldText, fieldValue, humanize, parseField } from "./fields";
import { workbenchAPI } from "./http";
import { Measurements } from "./Measurements";
import { SourceReview } from "./SourceReview";
import { LatestRunner } from "./runner";
import { collectSample, type SeedSample } from "./sample";
import "./workbench.css";

const defaultAPI = workbenchAPI();
const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));
const same = (left: unknown, right: unknown) => JSON.stringify(left) === JSON.stringify(right);

export function MapWorkbench({ api = defaultAPI }: { api?: WorkbenchAPI }) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loaded, setLoaded] = useState<Snapshot | null>(null);
  const [documents, setDocuments] = useState<Documents | null>(null);
  const [raw, setRaw] = useState<Record<string, string>>({});
  const [unfinished, setUnfinished] = useState<Set<string>>(new Set());
  const [mapType, setMapType] = useState<MapType>("mixed");
  const [mapSize, setMapSize] = useState<MapSize>("small");
  const [seed, setSeed] = useState("1");
  const [query, setQuery] = useState("");
  const [group, setGroup] = useState("");
  const [readOnly, setReadOnly] = useState(false);
  const [latest, setLatest] = useState<{ report: Report; request: RunRequest } | null>(null);
  const [accepted, setAccepted] = useState<{ report: Report; request: RunRequest } | null>(null);
  const [baseline, setBaseline] = useState<Report | null>(null);
  const [view, setView] = useState<"draft" | "saved">("draft");
  const [inspection, setInspection] = useState<(Inspection & { artifactId: string }) | null>(null);
  const [selected, setSelected] = useState<{ artifactId: string; id: string } | null>(null);
  const [zoom, setZoom] = useState(1);
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const [status, setStatus] = useState("Loading saved sources…");
  const [error, setError] = useState("");
  const [review, setReview] = useState<SaveReview | null>(null);
  const [sight, setSight] = useState<{
    artifactId: string;
    report: SightReport;
    roundTripMs: number;
    source: Pick<Report, "choice" | "fingerprint">;
  } | null>(null);
  const [sampleCount, setSampleCount] = useState(10);
  const [allCells, setAllCells] = useState(false);
  const [sample, setSample] = useState<SeedSample | null>(null);
  const [sampleRunning, setSampleRunning] = useState(false);
  const alive = useRef(true);
  const manual = useRef<AbortController | null>(null);
  const cancelledWork = useRef(0);
  const diagram = useRef<HTMLDivElement | null>(null);
  const artifactPins = useRef<string[]>([]);
  artifactPins.current = [
    ...new Set(
      [
        accepted?.report.artifactId,
        baseline?.status === "ok" ? baseline.artifactId : undefined,
      ].filter((id): id is string => !!id),
    ),
  ];
  const runner = useMemo(
    () =>
      new LatestRunner<RunRequest, Report>({
        delayMs: 350,
        run: (request, signal) => {
          if (alive.current) setStatus("Generating latest draft…");
          return api.generate({ ...request, retainedArtifactIds: artifactPins.current }, signal);
        },
        publish: (report, request) => {
          setLatest({ report, request });
          if (report.status === "ok") setAccepted({ report, request });
          setStatus(report.status === "ok" ? "Current draft admitted" : "Current draft refused");
          setError("");
        },
        fail: (failure) => {
          setError(errorText(failure));
          setStatus("Draft could not be inspected");
        },
      }),
    [api],
  );

  useEffect(() => {
    alive.current = true;
    const controller = new AbortController();
    void api.snapshot(controller.signal).then(
      (value) => {
        if (alive.current) {
          setSnapshot(value);
          setDocuments(value.documents);
        }
      },
      (failure) => {
        if (!controller.signal.aborted) setError(errorText(failure));
      },
    );
    return () => {
      alive.current = false;
      controller.abort();
      manual.current?.abort();
      runner.dispose();
    };
  }, [api, runner]);

  const choice = useMemo<MapChoice | null>(() => {
    const text = canonicalSeed(seed);
    return text === null ? null : { type: mapType, size: mapSize, seed: text };
  }, [mapType, mapSize, seed]);
  const draft = useMemo<Draft | null>(
    () => (snapshot && documents ? { revision: snapshot.revision, documents } : null),
    [snapshot, documents],
  );
  const currentInput = useRef({ draft, choice, latest, valid: unfinished.size === 0 });
  currentInput.current = { draft, choice, latest, valid: unfinished.size === 0 };

  useEffect(() => {
    setReview(null);
    setBaseline(null);
    setInspection(null);
    manual.current?.abort();
    if (!draft || !choice || unfinished.size) {
      runner.invalidate();
      if (draft) setStatus("Finish input before regeneration");
      return;
    }
    setStatus("Waiting for edits to settle…");
    runner.request({ purpose: "preview", retainedArtifactIds: [], draft, choice });
  }, [runner, draft, choice, unfinished]);

  const current =
    unfinished.size === 0 &&
    !!latest &&
    draft !== null &&
    choice !== null &&
    latest.request.draft.documents === draft.documents &&
    same(latest.report.choice, choice);
  const shown = view === "saved" ? baseline : accepted?.report;
  const selectedId = selected?.artifactId === shown?.artifactId ? selected?.id : null;
  const picture = shown && inspection?.artifactId === shown.artifactId ? inspection : shown;
  const svg = picture?.svg;
  const selectedCrop = shown?.features?.find((feature) => feature.id === selectedId)?.crop;
  const layers = useMemo(() => {
    if (!svg) return [];
    const dom = new DOMParser().parseFromString(svg, "image/svg+xml");
    return [
      ...new Set(
        Array.from(dom.querySelectorAll("[data-layer]"))
          .map((node) => node.getAttribute("data-layer")!)
          .filter(Boolean),
      ),
    ];
  }, [svg]);
  useEffect(() => {
    for (const node of diagram.current?.querySelectorAll<SVGElement>("[data-layer]") ?? [])
      node.style.display = hidden.has(node.dataset.layer!) ? "none" : "";
    for (const node of diagram.current?.querySelectorAll<SVGElement>("[data-feature-id]") ?? [])
      node.dataset.selected = String(node.dataset.featureId === selectedId);
  }, [svg, selectedId, hidden]);

  const fields = useMemo(
    () =>
      (snapshot?.fields ?? []).filter((field) => {
        const searchable =
          `${field.label} ${field.group} ${field.path.join(".")} ${field.description}`.toLowerCase();
        return (
          (readOnly || field.editable) &&
          (!group || field.group.startsWith(group) || field.path.join(".").startsWith(group)) &&
          searchable.includes(query.toLowerCase())
        );
      }),
    [snapshot, group, query, readOnly],
  );
  const groups = useMemo(
    () => [...new Set((snapshot?.fields ?? []).map((field) => field.group))].sort(),
    [snapshot],
  );

  function change(field: Field, text: string) {
    if (!documents) return;
    setRaw((value) => ({ ...value, [field.id]: text }));
    try {
      const value = parseField(text, fieldValue(documents, field), field);
      setUnfinished((old) => {
        if (!old.has(field.id)) return old;
        const next = new Set(old);
        next.delete(field.id);
        return next;
      });
      if (!same(value, fieldValue(documents, field)))
        setDocuments(editField(documents, field, value));
    } catch {
      setUnfinished((old) => (old.has(field.id) ? old : new Set(old).add(field.id)));
    }
  }

  async function operation(action: (signal: AbortSignal) => Promise<void>) {
    manual.current?.abort();
    const controller = new AbortController();
    manual.current = controller;
    const cancellation = cancelledWork.current;
    setError("");
    try {
      await runner.cancel();
      if (controller.signal.aborted) return;
      await action(controller.signal);
    } catch (failure) {
      if (!controller.signal.aborted && alive.current) setError(errorText(failure));
    } finally {
      if (manual.current === controller) {
        manual.current = null;
        const now = currentInput.current;
        if (
          alive.current &&
          !controller.signal.aborted &&
          cancellation === cancelledWork.current &&
          now.draft &&
          now.choice &&
          now.valid &&
          (now.latest?.request.draft.documents !== now.draft.documents ||
            !same(now.latest?.report.choice, now.choice))
        )
          runner.request({
            purpose: "preview",
            retainedArtifactIds: [],
            draft: now.draft,
            choice: now.choice,
          });
      }
    }
  }

  function download(name: string, value: unknown) {
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(value, null, 2)], { type: "application/json" }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download = name;
    link.click();
    URL.revokeObjectURL(url);
  }

  async function compare() {
    if (!snapshot || !choice) return;
    if (
      accepted?.request.draft.revision === snapshot.revision &&
      same(accepted.request.draft.documents, snapshot.documents) &&
      same(accepted.report.choice, choice)
    ) {
      setBaseline(accepted.report);
      setView("saved");
      return;
    }
    await operation(async (signal) => {
      setStatus("Inspecting saved defaults on the same seed…");
      const result = await api.generate(
        { purpose: "preview", retainedArtifactIds: artifactPins.current, draft: snapshot, choice },
        signal,
      );
      if (signal.aborted) return;
      setBaseline(result);
      setView("saved");
      setStatus(result.status === "ok" ? "Saved baseline admitted" : "Saved baseline refused");
    });
  }

  async function inspect(crop?: string) {
    if (!shown?.artifactId) return;
    const artifactId = shown.artifactId;
    await operation(async (signal) => {
      const result = await api.inspect(artifactId, crop, signal);
      if (!signal.aborted) {
        setInspection({ artifactId, ...result });
        setZoom(1);
      }
    });
  }

  async function measureSight() {
    if (!shown?.artifactId) return;
    const artifactId = shown.artifactId;
    await operation(async (signal) => {
      setStatus("Measuring sampled openness…");
      const start = performance.now();
      const result = await api.sight(artifactId, signal);
      if (!signal.aborted) {
        setSight({
          artifactId,
          report: result,
          roundTripMs: performance.now() - start,
          source: { choice: shown.choice, fingerprint: shown.fingerprint },
        });
        setStatus("Openness report ready");
      }
    });
  }

  async function runSample() {
    if (
      !draft ||
      !choice ||
      unfinished.size ||
      !Number.isInteger(sampleCount) ||
      sampleCount < 1 ||
      sampleCount > 10
    )
      return;
    const captured = draft;
    const cells = allCells
      ? MAP_TYPES.flatMap((type) => MAP_SIZES.map((size) => ({ type, size })))
      : [{ type: choice.type, size: choice.size }];
    const choices = cells.flatMap((cell) =>
      Array.from({ length: sampleCount }, (_, index) => ({
        ...cell,
        seed: String(BigInt.asUintN(64, BigInt(choice.seed) + BigInt(index))),
      })),
    );
    await operation(async (signal) => {
      setSampleRunning(true);
      try {
        const result = await collectSample(api, captured, choices, signal, (progress) => {
          if (alive.current) setSample(progress);
        });
        if (alive.current) setSample(result);
      } finally {
        if (alive.current) {
          setSampleRunning(false);
          setStatus(
            signal.aborted
              ? "Sample cancelled; completed outcomes retained"
              : "Seed sample complete",
          );
        }
      }
    });
  }

  async function reload(discard: boolean) {
    await operation(async (signal) => {
      const value = await api.snapshot(signal);
      if (signal.aborted) return;
      if (discard) {
        setSnapshot(value);
        setDocuments(value.documents);
        setRaw({});
        setUnfinished(new Set());
        setLoaded(null);
        setView("draft");
      } else {
        setLoaded(value);
        setStatus("Sources loaded; current draft retained");
      }
    });
  }

  function reapply() {
    if (!loaded || !snapshot || !documents) return;
    for (const id of unfinished) {
      const field = snapshot.fields.find((field) => field.id === id);
      const target = loaded.fields.find((field) => field.id === id && field.editable);
      if (!target || fieldValue(loaded.documents, target) === undefined) {
        setError(
          `The unfinished field ${field?.label ?? id} no longer exists. Undo it before reapplying.`,
        );
        return;
      }
    }
    let next = loaded.documents;
    for (const field of snapshot.fields) {
      if (
        !field.editable ||
        same(fieldValue(snapshot.documents, field), fieldValue(documents, field))
      )
        continue;
      const target = loaded.fields.find((value) => value.id === field.id && value.editable);
      if (!target || fieldValue(loaded.documents, target) === undefined) {
        setError(`The edited field ${field.label} no longer exists. Review or discard the draft.`);
        return;
      }
      next = editField(next, target, fieldValue(documents, field)!);
    }
    setSnapshot(loaded);
    setDocuments(next);
    setRaw((old) => Object.fromEntries(Object.entries(old).filter(([id]) => unfinished.has(id))));
    setLoaded(null);
    setView("draft");
  }

  async function preview() {
    if (!draft || unfinished.size) return;
    await operation(async (signal) => {
      const value = await api.preview(draft, signal);
      if (!signal.aborted) setReview(value);
    });
  }

  async function save() {
    if (!review) return;
    await operation(async (signal) => {
      const value = await api.save(review, signal);
      if (signal.aborted) return;
      setSnapshot(value);
      setDocuments(value.documents);
      setRaw({});
      setUnfinished(new Set());
      setLoaded(null);
      setReview(null);
      setView("draft");
      setStatus("Saved defaults; new battles use the published values");
    });
  }

  const shownStale = view === "draft" && (!current || latest?.report.status !== "ok");
  return (
    <main className="map-workbench">
      <header className="mw-header">
        <a href="/">Menu</a>
        <h1>Map workbench</h1>
        <span>Generation & validation</span>
        <button onClick={() => void reload(false)}>Reload sources</button>
      </header>
      <div className="mw-toolbar">
        <label>
          Map type
          <select
            aria-label="Map type"
            value={mapType}
            onChange={(event) => setMapType(event.target.value as MapType)}
          >
            {MAP_TYPES.map((type) => (
              <option key={type}>{type}</option>
            ))}
          </select>
        </label>
        <label>
          Size
          <select
            aria-label="Map size"
            value={mapSize}
            onChange={(event) => setMapSize(event.target.value as MapSize)}
          >
            {MAP_SIZES.map((size) => (
              <option key={size}>{size}</option>
            ))}
          </select>
        </label>
        <label>
          Seed
          <input
            aria-label="Map seed"
            value={seed}
            onChange={(event) => setSeed(event.target.value)}
          />
        </label>
        <button onClick={() => setSeed(newSeed())}>New seed</button>
        <button
          disabled={!draft || !choice || !!unfinished.size}
          onClick={() => {
            setStatus("Generating current draft…");
            runner.request({
              purpose: "preview",
              retainedArtifactIds: [],
              draft: draft!,
              choice: choice!,
            });
          }}
        >
          Regenerate
        </button>
        <button
          onClick={() => {
            ++cancelledWork.current;
            manual.current?.abort();
            void runner.cancel();
            setStatus("Cancelled; previous result retained");
          }}
        >
          Cancel work
        </button>
      </div>
      <div className="mw-status" role="status">
        {status}
        {!choice && <strong> · Seed must be a u64 decimal integer</strong>}
        {unfinished.size > 0 && <strong> · {unfinished.size} unfinished field(s)</strong>}
      </div>
      {error && (
        <p className="mw-error" role="alert">
          {error}
        </p>
      )}
      {loaded && (
        <div className="mw-reload">
          <p>
            New source revision {loaded.revision.slice(0, 12)} loaded. Your draft is retained.
            Reapplying keeps outside changes to fields you did not edit; your edited fields take
            precedence.
          </p>
          <button onClick={reapply}>Reapply my edits to new sources</button>
          <button onClick={() => void reload(true)}>Discard draft and reload</button>
        </div>
      )}
      <div className="mw-body">
        <aside className="mw-controls">
          <label className="mw-search">
            Find a rule
            <input
              aria-label="Search rules"
              placeholder="Apartments, river, fairness…"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
          <label>
            Rule group
            <select
              aria-label="Rule group"
              value={group}
              onChange={(event) => setGroup(event.target.value)}
            >
              <option value="">All rules</option>
              {groups.map((name) => (
                <option key={name} value={name}>
                  {humanize(name)}
                </option>
              ))}
              {group && !groups.includes(group) && <option value={group}>{humanize(group)}</option>}
            </select>
          </label>
          <label className="mw-checkbox">
            <input
              type="checkbox"
              checked={readOnly}
              onChange={(event) => setReadOnly(event.target.checked)}
            />
            Include physical and managed values
          </label>
          {group.startsWith("districts.") && (
            <p className="mw-scope">
              Changes affect this district kind wherever this preset is used.
            </p>
          )}
          <div className="mw-fields">
            {fields.length > 5 && (
              <p className="mw-scroll-hint">Scroll here for more matching rules.</p>
            )}
            {fields.map((field) => {
              const value = documents && fieldValue(documents, field);
              const saved = snapshot && fieldValue(snapshot.documents, field);
              const changed = !same(value, saved);
              const text = raw[field.id] ?? fieldText(value ?? undefined, field);
              return (
                <section className={`mw-field ${changed ? "mw-changed" : ""}`} key={field.id}>
                  <div className="mw-field-title">
                    <label htmlFor={`field-${field.id}`}>{field.label}</label>
                    <span>{field.role}</span>
                  </div>
                  <div className="mw-field-value">
                    {field.editable ? (
                      typeof value === "boolean" ? (
                        <input
                          id={`field-${field.id}`}
                          type="checkbox"
                          checked={value}
                          onChange={(event) => change(field, String(event.target.checked))}
                        />
                      ) : (
                        <input
                          id={`field-${field.id}`}
                          value={text}
                          aria-invalid={unfinished.has(field.id)}
                          onChange={(event) => change(field, event.target.value)}
                        />
                      )
                    ) : (
                      <output>{text}</output>
                    )}
                    <span>{field.unit}</span>
                    <small>Saved {fieldText(saved ?? undefined, field)}</small>
                  </div>
                  <p>{field.description}</p>
                  <code>
                    {field.document}.{field.path.join(".")}
                  </code>
                  {(changed || unfinished.has(field.id)) && (
                    <button
                      className="mw-reset"
                      onClick={() => {
                        setDocuments(editField(documents!, field, saved!));
                        setRaw((old) => {
                          const next = { ...old };
                          delete next[field.id];
                          return next;
                        });
                        setUnfinished((old) => {
                          const next = new Set(old);
                          next.delete(field.id);
                          return next;
                        });
                      }}
                    >
                      Undo edit
                    </button>
                  )}
                </section>
              );
            })}
            {snapshot && fields.length === 0 && <p>No matching fields.</p>}
          </div>
        </aside>
        <section className="mw-inspection">
          <div className="mw-view-tools">
            <button aria-pressed={view === "draft"} onClick={() => setView("draft")}>
              Draft plan
            </button>
            <button
              disabled={!snapshot || !choice || sampleRunning}
              aria-pressed={view === "saved"}
              onClick={() => (baseline ? setView("saved") : void compare())}
            >
              Saved baseline · same seed
            </button>
            <button
              disabled={!svg || zoom >= 8}
              onClick={() => setZoom((value) => Math.min(8, value * 1.5))}
            >
              Zoom in
            </button>
            <button
              disabled={!svg || zoom <= 1}
              onClick={() => setZoom((value) => Math.max(1, value / 1.5))}
            >
              Zoom out
            </button>
            <button
              disabled={shown?.status !== "ok" || !shown.artifactId || sampleRunning}
              onClick={() => void inspect()}
            >
              Whole map
            </button>
            <button
              disabled={!selectedCrop || !shown?.artifactId || sampleRunning}
              onClick={() => void inspect(selectedCrop!)}
            >
              Inspect selection
            </button>
          </div>
          <p className="mw-plan-label">
            {view === "saved" ? "Saved defaults" : "Draft"} ·{" "}
            {shown
              ? `${shown.choice.type} ${shown.choice.size} · seed ${shown.choice.seed}`
              : "No admitted plan yet"}
            {shownStale && <strong> · OLDER RESULT — current draft not admitted</strong>}
            {shown && <small> · input {shown.fingerprint.slice(0, 12)}</small>}
          </p>
          {layers.length > 0 && (
            <div className="mw-layers">
              {layers.map((layer) => (
                <label key={layer}>
                  <input
                    type="checkbox"
                    checked={!hidden.has(layer)}
                    onChange={(event) =>
                      setHidden((old) => {
                        const next = new Set(old);
                        if (event.target.checked) next.delete(layer);
                        else next.add(layer);
                        return next;
                      })
                    }
                  />
                  {humanize(layer)}
                </label>
              ))}
            </div>
          )}
          {picture?.summary && (
            <div className="mw-plan-summary">
              {picture.summary.map((line, index) => (
                <p key={index}>{line}</p>
              ))}
            </div>
          )}
          <div className="mw-plan-scroll">
            <div
              ref={diagram}
              data-testid="map-plan"
              className="mw-plan"
              style={{ width: `${zoom * 100}%`, height: `${zoom * 100}%` }}
              onClick={(event) => {
                const node =
                  event.target instanceof Element
                    ? event.target.closest("[data-rule-group]")
                    : null;
                if (node && shown?.artifactId) {
                  setGroup(node.getAttribute("data-rule-group")!);
                  setQuery("");
                  const id = node.getAttribute("data-feature-id");
                  setSelected(id ? { artifactId: shown.artifactId, id } : null);
                }
              }}
              dangerouslySetInnerHTML={{
                __html:
                  svg ||
                  (shown?.status === "refused"
                    ? '<div class="mw-empty">Saved baseline refused. Choose Draft plan to inspect the last admitted map.</div>'
                    : '<div class="mw-empty">The plan will appear after generation. Draft edits do not save automatically.</div>'),
              }}
            />
          </div>
          {picture?.legend && (
            <div className="mw-plan-legend">
              {picture.legend.map((item) => (
                <span key={item.label}>
                  <i style={{ backgroundColor: item.color }} />
                  {item.label}
                </span>
              ))}
            </div>
          )}
          {shown?.features && (
            <label className="mw-feature-picker">
              Inspect a feature
              <select
                aria-label="Inspect feature"
                value={selectedId ?? ""}
                onChange={(event) => {
                  const feature = shown.features!.find((item) => item.id === event.target.value);
                  setSelected(
                    feature && shown.artifactId
                      ? { artifactId: shown.artifactId, id: feature.id }
                      : null,
                  );
                  if (feature) {
                    setGroup(feature.group);
                    setQuery("");
                  }
                }}
              >
                <option value="">Choose settlement, district or bridge</option>
                {shown.features.map((feature) => (
                  <option key={feature.id} value={feature.id}>
                    {feature.label}
                  </option>
                ))}
              </select>
            </label>
          )}
          {latest?.report.status === "refused" && (
            <div className="mw-refusal">
              <h2>Requested draft refused</h2>
              <p>This seed was retained. Valid settings may still be saved.</p>
              <ul>
                {latest.report.diagnostics.map((row, index) => (
                  <li key={index}>
                    <strong>{row.feature || row.code}</strong> · {row.message}
                    <code>{row.location}</code>
                  </li>
                ))}
              </ul>
              {latest.report.artifactId && (
                <button
                  onClick={() =>
                    void operation(async (signal) =>
                      download(
                        "map-refused-inputs.json",
                        await api.export(latest.report.artifactId!, signal),
                      ),
                    )
                  }
                >
                  Export refused inputs
                </button>
              )}
            </div>
          )}
          {view === "saved" && baseline?.status === "refused" && (
            <div className="mw-refusal">
              Saved baseline refused: {baseline.diagnostics.map((row) => row.message).join("; ")}
            </div>
          )}
          {shown && (
            <>
              <p className="mw-result-origin">
                {shownStale
                  ? "Older admitted result"
                  : view === "saved"
                    ? "Saved baseline"
                    : "Current admitted result"}{" "}
                · {shown.choice.type} {shown.choice.size} · seed {shown.choice.seed} · input{" "}
                {shown.fingerprint.slice(0, 12)}
              </p>
              <Measurements report={shown} />
            </>
          )}
          <div className="mw-analysis">
            <button
              disabled={shown?.status !== "ok" || !shown.artifactId || sampleRunning}
              onClick={() => void measureSight()}
            >
              Measure openness
            </button>
            <span>Sampled sight runs on demand to keep editing responsive.</span>
            {shown?.artifactId && (
              <button
                disabled={sampleRunning}
                onClick={() =>
                  void operation(async (signal) =>
                    download("map-input-report.json", await api.export(shown.artifactId!, signal)),
                  )
                }
              >
                Export tested inputs
              </button>
            )}
          </div>
          {sight && (
            <div className="mw-sight">
              <h2>
                Sampled openness{sight.artifactId !== shown?.artifactId ? " · older artifact" : ""}
              </h2>
              <p className="mw-result-origin">
                {sight.artifactId !== shown?.artifactId || shownStale
                  ? "Older measured result · "
                  : ""}
                {sight.source.choice.type} {sight.source.choice.size} · seed{" "}
                {sight.source.choice.seed} · input {sight.source.fingerprint.slice(0, 12)}
              </p>
              <p>
                {sight.report.status === "measured"
                  ? `Median ${((sight.report.median ?? 0) * 100).toFixed(1)}% · target ${(sight.report.target * 100).toFixed(1)}% · ${sight.report.samples} samples, ${sight.report.step_m} m spacing`
                  : sight.report.reason || "No eligible samples"}
              </p>
              <p>
                Additional report round trip {sight.roundTripMs.toFixed(0)} ms. This measures
                sampled infantry openness, separately from the continuous physical certificate.
              </p>
            </div>
          )}
        </section>
      </div>
      <section className="mw-publication">
        <h2>Keep these defaults</h2>
        <p>
          Save replaces the reviewed fixture files in this checkout. New battles use the saved
          values; running battles keep their captured inputs.
        </p>
        <button disabled={!draft || !!unfinished.size} onClick={() => void preview()}>
          Review save
        </button>
        {review && (
          <div className="mw-review">
            <h3>Exact source replacements</h3>
            {latest?.report.status === "refused" && (
              <p>Selected seed refused. Saving valid settings is permitted.</p>
            )}
            {review.diagnostics.length > 0 && (
              <ul>
                {review.diagnostics.map((row, index) => (
                  <li key={index}>{row.message}</li>
                ))}
              </ul>
            )}
            {review.files.map((file) => (
              <details key={file.path}>
                <summary>{file.path}</summary>
                <SourceReview file={file} />
              </details>
            ))}
            <button disabled={review.diagnostics.length > 0} onClick={() => void save()}>
              Save reviewed defaults
            </button>
            <button onClick={() => setReview(null)}>Close review</button>
          </div>
        )}
      </section>
      <section className="mw-sample">
        <h2>Check several seeds</h2>
        <p>
          The sample pins its inputs and retains refusals. Saving does not require a successful
          sample.
        </p>
        <label>
          Seeds per cell
          <input
            aria-label="Sample seed count"
            type="number"
            min="1"
            max="10"
            value={sampleCount}
            onChange={(event) => setSampleCount(Number(event.target.value))}
          />
        </label>
        <label className="mw-checkbox">
          <input
            type="checkbox"
            checked={allCells}
            onChange={(event) => setAllCells(event.target.checked)}
          />
          All nine type/size cells
        </label>
        <button
          disabled={!draft || !choice || !!unfinished.size || sampleRunning}
          onClick={() => void runSample()}
        >
          Run seed sample
        </button>
        {sampleRunning && <button onClick={() => manual.current?.abort()}>Cancel sample</button>}
        {sample && (
          <>
            <p>
              {sample.rows.length} / {sample.choices.length} requested outcomes ·{" "}
              {sample.cancelled
                ? sample.rows.length < sample.choices.length
                  ? "cancelled; remaining seeds unattempted"
                  : "outcomes completed; receipt capture cancelled"
                : sample.complete
                  ? "complete"
                  : "running"}
              {draft && sample.input.documents !== draft.documents ? " · older input snapshot" : ""}
            </p>
            <button onClick={() => download("map-seed-sample.json", sample)}>
              Export sample summary
            </button>
            {sample.receiptErrors.length > 0 && (
              <p className="mw-error">
                Source export incomplete: {sample.receiptErrors.join("; ")}
              </p>
            )}
            <div className="mw-sample-table">
              <table>
                <thead>
                  <tr>
                    <th>Map</th>
                    <th>Seed</th>
                    <th>Map admission</th>
                    <th>Buildings</th>
                    <th>Encounter</th>
                    <th>Diagnostics</th>
                  </tr>
                </thead>
                <tbody>
                  {sample.rows.map((row, index) => (
                    <tr key={index}>
                      <td>
                        {row.choice.type} {row.choice.size}
                      </td>
                      <td>{row.choice.seed}</td>
                      <td>{row.status}</td>
                      <td>{String(row.counts?.buildings ?? "—")}</td>
                      <td>{String(row.encounter?.status ?? "—")}</td>
                      <td>{row.diagnostics.map((diagnostic) => diagnostic.message).join("; ")}</td>
                    </tr>
                  ))}
                  {sample.cancelled &&
                    sample.choices.slice(sample.rows.length).map((choice) => (
                      <tr key={`${choice.type}-${choice.size}-${choice.seed}`}>
                        <td>
                          {choice.type} {choice.size}
                        </td>
                        <td>{choice.seed}</td>
                        <td colSpan={4}>Unattempted</td>
                      </tr>
                    ))}
                </tbody>
              </table>
            </div>
          </>
        )}
      </section>
    </main>
  );
}

export default MapWorkbench;
