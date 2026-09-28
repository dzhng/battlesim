import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import {
  REST_ARTICULATION,
  PITCH_LIMITS,
  type Articulation,
} from "@packages/scene-assets/src/articulation";
import {
  SIDES,
  TEXTURE_CHANNELS,
  UNIT_BUNDLE_KIND,
  type Side,
  type TextureChannel,
  type UnitKind,
} from "@packages/scene-assets/src/schema";
import { SCENERY_KINDS } from "@packages/scene-assets/src/scenery";
import type { LooseOptions } from "@packages/scene-assets/src/loose";
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";
import {
  poseFrameInstances,
  type ModelInstance,
  type ModelPose,
} from "@packages/battle-renderer/src/models/modelInstances";
import { PoseDriver, type UnitKindName } from "@packages/battle-renderer/src/models/poseDriver";
import type { ImpostorAtlas } from "@packages/battle-renderer/src/models/impostor";
import { LabViewport, type ViewportGpu } from "../LabViewport";
import { benchOverlay, benchWorld, posedSockets } from "../workbench/benchWorld";
import { beatAt, feedAt, replayLength } from "../workbench/feedReplay";
import { paletteError } from "../workbench/paletteCheck";
import {
  atlasCanvas,
  figureSpot,
  framingBounds,
  renderSheet,
  sheetPose,
  textureSheet,
} from "../workbench/sheet";
import {
  catalogModel,
  INFANTRY_LOOPS,
  loadCatalog,
  loadDropped,
  sideTint,
  type LoadedModel,
} from "../workbench/sources";
import { halfTrack, mountRoles, villagePose } from "../poseFeed";
import village from "@fixtures/village.json";
import {
  WORKBENCH_CAMERA,
  WORKBENCH_VIEWS,
  viewCamera,
  type WorkbenchView,
} from "../workbench/views";
import { useFeed, type FeedSource } from "../feed";

/** A model with no side keeps its authored colours. */
const NO_TINT = [1, 1, 1] as const;

// The model workbench: drop a GLB (or pick a catalog appearance) and, within
// seconds, see the validator's findings and our own production render — the
// battle frame, light and pose paths — with every view needed to judge a
// model or an animation: named views, a 1.8 m figure, the simulation's hit
// box, socket gizmos, a clip and phase scrubber, a feed replay through the
// pose driver, stats, sheets and the impostor bake.

const EMPTY: WorldMeshes = { opaque: new Float32Array(0), translucent: new Float32Array(0) };
const DEG = Math.PI / 180;
const UNITS: UnitKind[] = ["rifle", "recon", "at", "tank", "supply", "jeep", "building", "scenery"];

type PoseMode = "manual" | "feed";

interface WorkbenchHandle {
  drop(name: string, bytes: Uint8Array, options?: LooseOptions): Promise<void>;
  select(name: string): Promise<void>;
  state(): {
    model: string | null;
    unit: UnitKind | null;
    findings: { code: string; severity: string; message: string }[];
    stats: unknown;
    loadMs: number;
    catalog: string[];
  };
  setView(view: WorkbenchView): Promise<void>;
  setPose(pose: ModelPose, tier?: number): Promise<void>;
  setFeed(time: number | null): Promise<void>;
  show(options: { figure?: boolean; hitBox?: boolean; sockets?: boolean }): Promise<void>;
  /** Which army's tint the model wears (its tint-masked surfaces). */
  setSide(side: Side): Promise<void>;
  paletteError(): Promise<number>;
  bakeImpostor(): Promise<{
    hash: string;
    width: number;
    height: number;
    albedo: string;
    normal: string;
  }>;
  sheet(): Promise<{
    contact: string;
    strips: { name: string; png: string }[];
    surface: string;
    textures: string | null;
    stats: unknown;
  }>;
  reloadCatalog(): Promise<string[]>;
  models(): readonly ModelInstance[];
}

declare global {
  interface Window {
    __workbench?: WorkbenchHandle;
  }
}

const toBase64 = (bytes: Uint8Array) => {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000)
    s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
};

function initialPose(model: LoadedModel | null): ModelPose | null {
  if (!model) return null;
  const bundle = model.installed.appearances.get(model.name)?.bundle;
  if (!bundle) return null;
  const skeleton =
    bundle.kind === "skinned" ? (model.installed.skeletons.get(bundle.skeleton) ?? null) : null;
  return sheetPose(bundle, skeleton);
}

export default function Workbench() {
  const params = useMemo(() => new URLSearchParams(window.location.search), []);
  const [catalog, setCatalog] = useState<InstalledAppearances | null>(null);
  const [model, setModel] = useState<LoadedModel | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [pose, setPose] = useState<ModelPose | null>(null);
  const [tier, setTier] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [mode, setMode] = useState<PoseMode>("manual");
  const [feedTime, setFeedTime] = useState(0);
  const [show, setShow] = useState({ figure: true, hitBox: true, sockets: true });
  // Material texture channels drawn, and whether the texture preview is open.
  const [channels, setChannels] = useState<Record<TextureChannel, boolean>>({
    albedo: true,
    normal: true,
    orm: true,
  });
  const [showTextures, setShowTextures] = useState(false);
  const [view, setView] = useState<WorkbenchView>("q-front");
  const [side, setSide] = useState<Side>(params.get("side") === "red" ? "red" : "blue");
  const [impostor, setImpostor] = useState<ImpostorAtlas | null>(null);
  const impostorFeed = useFeed(impostor);
  const [sheetUrl, setSheetUrl] = useState<string | null>(null);
  const [options, setOptions] = useState<{
    unit: UnitKind | "auto";
    scenery: string;
    yaw: "auto" | number;
  }>({
    unit: "auto",
    scenery: "tree",
    yaw: "auto",
  });
  const dropped = useRef<{ name: string; bytes: Uint8Array } | null>(null);
  const gpu = useRef<ViewportGpu | null>(null);
  const [ready, setReady] = useState(false);
  const [dragging, setDragging] = useState(false);

  const bundle = model?.installed.appearances.get(model.name)?.bundle ?? null;
  const skeleton =
    bundle?.kind === "skinned" ? (model!.installed.skeletons.get(bundle.skeleton) ?? null) : null;
  const unitKind = model?.unit ?? null;
  const tint = useMemo(() => (model ? sideTint(model, side) : undefined), [model, side]);
  const framing = useMemo(() => (model && bundle ? framingBounds(model) : null), [model, bundle]);

  // The pose driver, fed by the replay; rebuilt per model. The model on the
  // bench plays every kind the replay drives, so its clips answer for all.
  const driver = useMemo(() => {
    const facts = skeleton;
    return new PoseDriver({
      mounts: mountRoles(village.mounts),
      halfTrack: halfTrack(village.physics, villagePose.gauge),
      pinned: village.suppression.collapse_level,
      feel: villagePose,
      clip: (_kind, name) => {
        const clip = facts?.clips.find((c) => c.name === name);
        return clip
          ? { duration: clip.duration, loop: clip.loop, stride_m: clip.stride_m ?? null }
          : null;
      },
    });
  }, [skeleton]);

  const feedKind: UnitKindName | null =
    unitKind && UNIT_BUNDLE_KIND[unitKind] !== "static" ? (unitKind as UnitKindName) : null;

  const feedModels = useCallback(
    (t: number): ModelInstance[] => {
      if (!model || !feedKind) return [];
      const feed = feedAt(feedKind, t);
      const frame = driver.update(feed);
      // The view follows the unit: its models are drawn relative to where it
      // started this frame, so the camera never loses a driving vehicle.
      const [ox, oy] = feed.units[0]?.position ?? [0, 0];
      const resolve = () => ({ appearance: model.name, tint: tint ?? NO_TINT });
      const posed = poseFrameInstances([], frame, resolve).map((m) => ({
        ...m,
        pose: { ...m.pose },
      }));
      const lying: ModelInstance[] = frame.corpses.map((c) => ({
        appearance: model.name,
        x: c.position[0],
        y: c.position[1],
        z: c.position[2],
        yaw: c.yaw,
        pose: { kind: "corpse" },
      }));
      return [...posed, ...lying].map((m) => ({
        ...m,
        x: m.x - ox,
        y: m.y - oy,
        tier,
        tint,
      }));
    },
    [driver, model, feedKind, tier, tint],
  );

  const models = useMemo<ModelInstance[]>(() => {
    if (!model || !pose) return [];
    if (mode === "feed") return feedModels(feedTime);
    return [{ appearance: model.name, x: 0, y: 0, z: 0, yaw: 0, pose, tier, tint }];
  }, [model, pose, tier, tint, mode, feedTime, feedModels]);

  // Per-frame animation: a playing clip, or the playing feed replay.
  const playRef = useRef({ playing, mode, last: 0 });
  playRef.current.playing = playing;
  playRef.current.mode = mode;
  /** Advances the clock only: the models follow through React state. */
  const animate = useCallback(
    (now: number) => {
      const state = playRef.current;
      const dt = state.last ? Math.min(0.1, (now - state.last) / 1000) : 0;
      state.last = now;
      if (!state.playing || !model || !pose) return null;
      if (state.mode === "feed" && feedKind) {
        setFeedTime((t) => (t + dt) % replayLength(feedKind));
        return null;
      }
      if (pose.kind === "skinned") {
        const clip = skeleton?.clips.find((c) => c.name === pose.clip);
        if (!clip) return null;
        const phase = clip.loop
          ? (pose.phase + dt / clip.duration) % 1
          : Math.min(1, pose.phase + dt / clip.duration);
        setPose({ ...pose, phase });
      }
      return null;
    },
    [model, pose, skeleton, feedKind],
  );

  const world = useMemo(
    () => benchWorld(framing && show.figure ? figureSpot(framing, view) : null),
    [framing, show.figure, view],
  );
  const worldFeed = useFeed(world);

  const overlay = useMemo<WorldMeshes>(() => {
    if (!bundle || !model || !framing) return EMPTY;
    const scale = Math.max(1, (framing.max[0] - framing.min[0]) / 3);
    const marks = models.map((m) =>
      benchOverlay(m, model.body, posedSockets(bundle, skeleton, m.pose), show, scale),
    );
    const size = marks.reduce((n, m) => n + m.opaque.length, 0);
    const opaque = new Float32Array(size);
    let at = 0;
    for (const m of marks) {
      opaque.set(m.opaque, at);
      at += m.opaque.length;
    }
    return { opaque, translucent: new Float32Array(0) };
  }, [bundle, model, models, skeleton, show, framing]);
  const overlayFeed = useFeed(overlay);

  const install = useCallback((next: LoadedModel) => {
    setModel(next);
    setPose(initialPose(next));
    setTier(0);
    setImpostor(null);
    setSheetUrl(null);
    setMode("manual");
    setFeedTime(0);
    setError(null);
  }, []);

  const loadBytes = useCallback(
    async (name: string, bytes: Uint8Array, opts: typeof options) => {
      setBusy(true);
      try {
        const next = await loadDropped(name, bytes, {
          unit: opts.unit === "auto" ? undefined : opts.unit,
          scenery: opts.unit === "scenery" ? opts.scenery : undefined,
          yaw: opts.yaw === "auto" ? undefined : opts.yaw,
          loops: INFANTRY_LOOPS,
        });
        install(next);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusy(false);
      }
    },
    [install],
  );

  const onFile = useCallback(
    async (file: File) => {
      const bytes = new Uint8Array(await file.arrayBuffer());
      dropped.current = { name: file.name, bytes };
      await loadBytes(file.name, bytes, options);
    },
    [loadBytes, options],
  );

  const reloadCatalog = useCallback(async (fresh = false) => {
    const installed = await loadCatalog(fresh);
    setCatalog(installed);
    return installed;
  }, []);

  // The runtime catalog, and `?bundle=` naming one of its appearances.
  useEffect(() => {
    void reloadCatalog()
      .then((installed) => {
        const name = params.get("bundle");
        if (!name) return;
        const found = catalogModel(installed, name);
        if (found) install(found);
        else setError(`no appearance "${name}" in the runtime catalog`);
      })
      .catch((e) => setError(`runtime catalog: ${e instanceof Error ? e.message : String(e)}`));
  }, [params, reloadCatalog, install]);

  // Hot reload: the dev server re-bakes on a source change and says so.
  useEffect(() => {
    const hot = import.meta.hot;
    if (!hot) return;
    const onRebaked = async (data: { ok: boolean; output: string }) => {
      if (!data.ok) {
        setError(`re-bake failed:\n${data.output}`);
        return;
      }
      const installed = await reloadCatalog(true);
      setModel((current) =>
        current?.source === "catalog"
          ? (catalogModel(installed, current.name) ?? current)
          : current,
      );
    };
    hot.on("assets:rebaked", onRebaked);
    return () => hot.off("assets:rebaked", onRebaked);
  }, [reloadCatalog]);

  const setCamera = useCallback(
    (v: WorkbenchView) => {
      setView(v);
      if (framing) window.__lab?.setCamera?.(viewCamera(v, framing));
    },
    [framing],
  );

  // Frame a new model in the current view.
  useEffect(() => {
    if (framing && ready) window.__lab?.setCamera?.(viewCamera(view, framing));
    // Only when the model changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [framing, ready]);

  const bake = useCallback(async () => {
    if (!model || !gpu.current) throw new Error("no model");
    const atlas = await gpu.current.frame().bakeImpostor(model.name);
    setImpostor(atlas);
    return atlas;
  }, [model]);

  const sheet = useCallback(async () => {
    if (!model || !gpu.current) throw new Error("no model");
    const result = await renderSheet(gpu.current.device, gpu.current.format, model, impostor, side);
    setSheetUrl(result.contact.toDataURL("image/png"));
    return result;
  }, [model, impostor, side]);

  // The channel switches live on the frame's models layer; a rebuilt frame
  // or a new model takes them again.
  useEffect(() => {
    if (ready) gpu.current?.frame().setTextureChannels(channels);
  }, [ready, channels, model]);
  const texturePreview = useMemo(() => {
    const bundle =
      showTextures && model ? model.installed.appearances.get(model.name)?.bundle : null;
    return bundle ? (textureSheet(model!.name, bundle)?.toDataURL("image/png") ?? null) : null;
  }, [showTextures, model]);

  // The scene harness's and the sheet CLI's hold on the workbench.
  const latest = useRef({ model, models, bake, sheet, catalog, skeleton });
  latest.current = { model, models, bake, sheet, catalog, skeleton };
  useEffect(() => {
    const drawn = () => window.__lab?.frame?.() ?? Promise.resolve();
    /** Resolve once `name` is installed on the GPU and drawn. */
    const installedOnGpu = async (name: string) => {
      for (let i = 0; i < 600; i++) {
        if (gpu.current?.frame().stats().models.installed.includes(name)) return drawn();
        await new Promise((r) => requestAnimationFrame(r));
      }
      throw new Error(`${name} did not install`);
    };
    const handle: WorkbenchHandle = {
      async drop(name, bytes, opts = {}) {
        dropped.current = { name, bytes };
        const next = await loadDropped(name, bytes, {
          loops: INFANTRY_LOOPS,
          ...opts,
        });
        install(next);
        await installedOnGpu(name);
      },
      async select(name) {
        const installed = latest.current.catalog ?? (await reloadCatalog());
        const found = catalogModel(installed, name);
        if (!found) throw new Error(`no appearance ${name}`);
        install(found);
        await installedOnGpu(name);
      },
      state() {
        const m = latest.current.model;
        return {
          model: m?.name ?? null,
          unit: m?.unit ?? null,
          findings: (m?.findings ?? []).flatMap((f) =>
            f.findings.map((x) => ({ code: x.code, severity: x.severity, message: x.message })),
          ),
          stats: m?.stats ?? null,
          loadMs: m?.loadMs ?? 0,
          catalog: [...(latest.current.catalog?.appearances.keys() ?? [])],
        };
      },
      async setView(v) {
        setCamera(v);
        await drawn();
      },
      async setPose(p, t = 0) {
        setMode("manual");
        setPlaying(false);
        setPose(p);
        setTier(t);
        await new Promise((r) => setTimeout(r, 0));
        await drawn();
      },
      async setFeed(time) {
        setPlaying(false);
        if (time === null) setMode("manual");
        else {
          setMode("feed");
          driver.reset();
          // Walk the driver up to `time` so gaits and phases are as if played.
          if (latest.current.model && feedKind)
            for (let t = 0; t < time; t += 1 / 30) driver.update(feedAt(feedKind, t));
          setFeedTime(time);
        }
        await new Promise((r) => setTimeout(r, 0));
        await drawn();
      },
      async show(next) {
        setShow((s) => ({ ...s, ...next }));
        await new Promise((r) => setTimeout(r, 0));
        await drawn();
      },
      async setSide(next) {
        setSide(next);
        await new Promise((r) => setTimeout(r, 0));
        await drawn();
      },
      async paletteError() {
        const m = latest.current.model;
        const instance = latest.current.models[0];
        if (!m || !instance || !gpu.current) return Infinity;
        return paletteError(gpu.current.frame(), m.installed, instance);
      },
      async bakeImpostor() {
        const atlas = await latest.current.bake();
        return {
          hash: atlas.hash,
          width: atlas.width,
          height: atlas.height,
          albedo: toBase64(atlas.albedo),
          normal: toBase64(atlas.normal),
        };
      },
      async sheet() {
        const result = await latest.current.sheet();
        return {
          contact: result.contact.toDataURL("image/png"),
          strips: result.strips.map((s) => ({
            name: s.name,
            png: s.canvas.toDataURL("image/png"),
          })),
          surface: result.surface.toDataURL("image/png"),
          textures: result.textures?.toDataURL("image/png") ?? null,
          stats: result.stats,
        };
      },
      async reloadCatalog() {
        const installed = await reloadCatalog(true);
        return [...installed.appearances.keys()];
      },
      models: () => latest.current.models,
    };
    window.__workbench = handle;
    return () => {
      if (window.__workbench === handle) delete window.__workbench;
    };
  }, [install, reloadCatalog, setCamera, driver, feedKind]);

  const initialCamera = useMemo(
    () => viewCamera("q-front", { min: [-1, -1, 0], max: [1, 1, 2] }),
    [],
  );

  const articulation = pose?.kind === "articulated" ? pose.articulation : REST_ARTICULATION;
  const setArticulation = (patch: Partial<Articulation>) =>
    setPose({ kind: "articulated", articulation: { ...articulation, ...patch } });
  const tiers = bundle
    ? bundle.kind === "skinned"
      ? bundle.tiers.length
      : bundle.kind === "articulated"
        ? (bundle.nodes[0]?.tiers.length ?? 0)
        : (bundle.states[0]?.tiers.length ?? 0)
    : 0;
  const findings = model?.findings ?? [];
  const errorCount = findings.reduce(
    (n, f) => n + f.findings.filter((x) => x.severity === "error").length,
    0,
  );

  return (
    <div
      style={{ position: "absolute", inset: 0 }}
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        const file = e.dataTransfer.files[0];
        if (file) void onFile(file);
      }}
      data-testid="workbench-drop"
    >
      <LabViewport
        fixture="workbench"
        world={worldFeed}
        overlay={overlayFeed}
        instances={[]}
        initialCamera={initialCamera}
        cameraConfig={WORKBENCH_CAMERA}
        appearances={model?.installed ?? null}
        models={models}
        frame={animate}
        onReady={(g) => {
          gpu.current = g;
          setReady(true);
        }}
        diagnostics={{ model: model?.name ?? null, view, mode }}
      />
      {dragging && <div className="wb-dropping">Drop a .glb to validate and render it</div>}
      <aside className="lab-panel wb-panel" data-testid="workbench-panel">
        <strong>Model workbench</strong>
        <label className="wb-file">
          <input
            type="file"
            accept=".glb"
            data-testid="workbench-file"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) void onFile(file);
            }}
          />
        </label>
        <div className="lab-row">
          <label>
            unit
            <select
              data-testid="workbench-unit"
              value={options.unit}
              onChange={(e) => {
                const next = { ...options, unit: e.target.value as UnitKind | "auto" };
                setOptions(next);
                if (dropped.current)
                  void loadBytes(dropped.current.name, dropped.current.bytes, next);
              }}
            >
              <option value="auto">auto</option>
              {UNITS.map((u) => (
                <option key={u} value={u}>
                  {u}
                </option>
              ))}
            </select>
          </label>
          {options.unit === "scenery" && (
            <label>
              kind
              <select
                data-testid="workbench-scenery"
                value={options.scenery}
                onChange={(e) => {
                  const next = { ...options, scenery: e.target.value };
                  setOptions(next);
                  if (dropped.current)
                    void loadBytes(dropped.current.name, dropped.current.bytes, next);
                }}
              >
                {Object.keys(SCENERY_KINDS).map((k) => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </select>
            </label>
          )}
          <label>
            basis yaw
            <select
              data-testid="workbench-yaw"
              value={String(options.yaw)}
              onChange={(e) => {
                const next = {
                  ...options,
                  yaw: e.target.value === "auto" ? ("auto" as const) : Number(e.target.value),
                };
                setOptions(next);
                if (dropped.current)
                  void loadBytes(dropped.current.name, dropped.current.bytes, next);
              }}
            >
              {["auto", "0", "90", "180", "270"].map((y) => (
                <option key={y} value={y}>
                  {y}
                </option>
              ))}
            </select>
          </label>
        </div>
        {catalog && catalog.appearances.size > 0 && (
          <label>
            catalog
            <select
              value={model?.source === "catalog" ? model.name : ""}
              onChange={(e) => {
                const found = catalogModel(catalog, e.target.value);
                if (found) install(found);
              }}
            >
              <option value="">—</option>
              {[...catalog.appearances.keys()].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </label>
        )}
        {busy && <div className="lab-hint">validating…</div>}
        {error && <pre className="lab-rejected wb-pre">{error}</pre>}
        {model && (
          <>
            <div data-testid="workbench-model">
              {model.name} · {model.scenery ?? model.unit} ({UNIT_BUNDLE_KIND[model.unit]}) ·{" "}
              {model.source === "catalog"
                ? "catalog"
                : `validated in ${Math.round(model.loadMs)} ms`}
            </div>
            <div className="lab-row" role="group" aria-label="views">
              {WORKBENCH_VIEWS.map((v) => (
                <button
                  key={v}
                  type="button"
                  aria-pressed={view === v}
                  onClick={() => setCamera(v)}
                >
                  {v}
                </button>
              ))}
            </div>
            <div className="lab-row">
              {(["figure", "hitBox", "sockets"] as const).map((k) => (
                <label key={k}>
                  <input
                    type="checkbox"
                    checked={show[k]}
                    onChange={(e) => setShow({ ...show, [k]: e.target.checked })}
                  />
                  {k === "figure" ? "1.8 m figure" : k === "hitBox" ? "hit box" : "sockets"}
                </label>
              ))}
              <label>
                side
                <select value={side} onChange={(e) => setSide(e.target.value as Side)}>
                  {SIDES.map((s) => (
                    <option key={s} value={s}>
                      {s}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                tier
                <select value={tier} onChange={(e) => setTier(Number(e.target.value))}>
                  {Array.from({ length: tiers }, (_, t) => (
                    <option key={t} value={t}>
                      LOD{t}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <div className="lab-row" role="group" aria-label="texture channels">
              textures
              {TEXTURE_CHANNELS.map((c) => (
                <label key={c}>
                  <input
                    type="checkbox"
                    data-testid={`workbench-channel-${c}`}
                    checked={channels[c]}
                    onChange={(e) => setChannels({ ...channels, [c]: e.target.checked })}
                  />
                  {c}
                </label>
              ))}
              <label>
                <input
                  type="checkbox"
                  checked={showTextures}
                  onChange={(e) => setShowTextures(e.target.checked)}
                />
                preview
              </label>
            </div>
            {showTextures &&
              (texturePreview ? (
                <img className="wb-textures" src={texturePreview} alt="material textures" />
              ) : (
                <div className="lab-hint">no textured materials</div>
              ))}
            <div className="lab-row">
              <button
                type="button"
                aria-pressed={mode === "manual"}
                onClick={() => setMode("manual")}
              >
                pose
              </button>
              {feedKind && (
                <button
                  type="button"
                  aria-pressed={mode === "feed"}
                  onClick={() => {
                    driver.reset();
                    setMode("feed");
                  }}
                >
                  feed replay
                </button>
              )}
              <button type="button" aria-pressed={playing} onClick={() => setPlaying(!playing)}>
                {playing ? "pause" : "play"}
              </button>
            </div>
            {mode === "feed" && feedKind && (
              <label>
                t {feedTime.toFixed(1)} s · {beatAt(feedKind, feedTime)}
                <input
                  type="range"
                  min={0}
                  max={replayLength(feedKind)}
                  step={0.05}
                  value={feedTime}
                  onChange={(e) => setFeedTime(Number(e.target.value))}
                />
              </label>
            )}
            {mode === "manual" && pose?.kind === "skinned" && (
              <>
                <label>
                  clip
                  <select
                    value={pose.clip}
                    onChange={(e) => setPose({ ...pose, clip: e.target.value, phase: 0 })}
                  >
                    {skeleton?.clips.map((c) => (
                      <option key={c.name} value={c.name}>
                        {c.name} {c.loop ? "(loop)" : ""} {c.duration.toFixed(2)} s
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  phase {pose.phase.toFixed(2)}
                  <input
                    type="range"
                    min={0}
                    max={1}
                    step={0.005}
                    value={pose.phase}
                    onChange={(e) => setPose({ ...pose, phase: Number(e.target.value) })}
                  />
                </label>
              </>
            )}
            {mode === "manual" && pose?.kind === "articulated" && (
              <>
                {(
                  [
                    ["turret_yaw", -180, 180, "turret yaw"],
                    [
                      "gun_pitch",
                      PITCH_LIMITS.gun[0] / DEG,
                      PITCH_LIMITS.gun[1] / DEG,
                      "gun pitch",
                    ],
                    ["hmg_yaw", -180, 180, "HMG yaw"],
                    [
                      "hmg_pitch",
                      PITCH_LIMITS.hmg[0] / DEG,
                      PITCH_LIMITS.hmg[1] / DEG,
                      "HMG pitch",
                    ],
                  ] as const
                ).map(([key, lo, hi, label]) => (
                  <label key={key}>
                    {label} {(articulation[key] / DEG).toFixed(0)}°
                    <input
                      type="range"
                      min={lo}
                      max={hi}
                      step={1}
                      value={articulation[key] / DEG}
                      onChange={(e) => setArticulation({ [key]: Number(e.target.value) * DEG })}
                    />
                  </label>
                ))}
                <label>
                  travel {articulation.travel_l.toFixed(2)} m
                  <input
                    type="range"
                    min={0}
                    max={4}
                    step={0.01}
                    value={articulation.travel_l}
                    onChange={(e) =>
                      setArticulation({
                        travel_l: Number(e.target.value),
                        travel_r: Number(e.target.value),
                      })
                    }
                  />
                </label>
                <label>
                  deploy {articulation.deploy.toFixed(2)}
                  <input
                    type="range"
                    min={0}
                    max={1}
                    step={0.01}
                    value={articulation.deploy}
                    onChange={(e) => setArticulation({ deploy: Number(e.target.value) })}
                  />
                </label>
              </>
            )}
            {mode === "manual" && pose?.kind === "static" && bundle?.kind === "static" && (
              <label>
                state
                <select
                  value={pose.state}
                  onChange={(e) => setPose({ kind: "static", state: e.target.value })}
                >
                  {bundle.states.map((s) => (
                    <option key={s.name}>{s.name}</option>
                  ))}
                </select>
              </label>
            )}
            <div className="lab-row">
              <button type="button" onClick={() => void bake()}>
                bake impostor
              </button>
              <button type="button" onClick={() => void sheet()}>
                sheet
              </button>
            </div>
          </>
        )}
        {!model && (
          <div className="lab-hint">
            Drop a .glb anywhere, choose one above, or open /workbench?bundle=name.
          </div>
        )}
      </aside>
      {model && (
        <aside className="lab-panel wb-side" data-testid="workbench-findings">
          <strong className={errorCount ? "lab-rejected" : undefined}>
            {errorCount} error(s),{" "}
            {findings.reduce((n, f) => n + f.findings.length, 0) - errorCount} warning(s)
          </strong>
          {findings.map((group) =>
            group.findings.map((f, i) => (
              <div
                key={`${group.from}-${i}`}
                className="wb-finding"
                data-code={f.code}
                data-severity={f.severity}
              >
                <span className={f.severity === "error" ? "lab-rejected" : "lab-hint"}>
                  {f.severity === "error" ? "ERROR" : "warn"} {f.code}
                </span>
                <div>{f.message}</div>
                <div className="lab-hint">fix: {f.fix}</div>
              </div>
            )),
          )}
          <div className="lab-hint" data-testid="workbench-footprint">
            simulation: {model.body.label}
          </div>
          <Stats model={model} />
          {impostor && (
            <div data-testid="workbench-impostor">
              <div className="lab-hint">
                impostor {impostor.width}×{impostor.height} · {impostor.hash.slice(0, 12)}
              </div>
              <AtlasView atlas={impostorFeed} />
            </div>
          )}
        </aside>
      )}
      {sheetUrl && (
        <div className="wb-sheet" onClick={() => setSheetUrl(null)}>
          <img src={sheetUrl} alt="contact sheet" />
        </div>
      )}
    </div>
  );
}

function Stats({ model }: { model: LoadedModel }) {
  const bundle = model.installed.appearances.get(model.name)?.bundle;
  if (!bundle) return null;
  const stats = model.stats;
  return (
    <div className="lab-hint" data-testid="workbench-stats">
      {stats?.tiers.length ? (
        <div>triangles per tier: {stats.tiers.map((t) => t.triangles).join(" / ")}</div>
      ) : null}
      {bundle.kind === "skinned" && <div>joints: {bundle.joints.length}</div>}
      {bundle.kind === "articulated" && <div>nodes: {bundle.nodes.length}</div>}
      {model.clipStats?.clips && (
        <div>
          clips:{" "}
          {model.clipStats.clips
            .map((c) => `${c.name}${c.loop ? "↻" : ""} ${c.duration.toFixed(2)}s`)
            .join(", ")}
        </div>
      )}
      <div>
        bounds: [{bundle.bounds.min.map((v) => v.toFixed(2)).join(", ")}] – [
        {bundle.bounds.max.map((v) => v.toFixed(2)).join(", ")}] m
      </div>
      {stats && <div>source: {(stats.source_bytes / 1024).toFixed(0)} KiB</div>}
    </div>
  );
}

/** The baked atlas's channels as canvases; fed (`useFeed`), since its pixels
 *  as a changing prop would fill React's development measures. */
function AtlasView({ atlas }: { atlas: FeedSource<ImpostorAtlas | null> }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const draw = (next: ImpostorAtlas | null) => {
      const el = ref.current;
      if (!el || !next) return;
      el.replaceChildren(
        atlasCanvas(next.width, next.height, next.albedo),
        atlasCanvas(next.width, next.height, next.normal),
      );
    };
    draw(atlas.current);
    return atlas.subscribe(draw);
  }, [atlas]);
  return <div ref={ref} className="wb-atlas" />;
}
