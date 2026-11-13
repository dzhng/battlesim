// The Armory's stage: the open card's unit standing on its spot in the yard,
// with the battle's idle life and idle sound, seen from the spot's camera.
// It is one `ShowcaseStage` for the page's life; each pick asks
// `ArmoryAppearances` for that unit's art and shows it. A planned card asks
// for nothing: the sheet carries it, and the yard stands empty. A failure
// stays on the stage, drawn by its own boundary, and never reaches the
// page's resources, so the list, the sheet and the menu stay usable.
import {
  Component,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import type { Faction, UnitCard } from "@packages/scene-assets/src/units";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { ShowcaseStage } from "@web/battle/render/showcaseStage";
import { appResources } from "../appResources";
import { useAppAudio } from "../AppAudio";
import { useLabLoading } from "../LabLoading";
import { useMenuShell } from "../menu/MenuShell";
import { presentedCamera } from "../presentedFrame";
import { ArmoryAppearances } from "./armoryAppearances";
import { framedOrbit } from "./armoryCamera";
import { publishProbe, type Shown, type Switch } from "./armoryProbe";
import { STAGE_LOOK, SQUAD_SEED, useShowground, useStageFielding } from "./stageInputs";
import { spotFor, yard, yardModels, yardScenery, yardWorld, type Spot } from "./yard";

interface StageProps {
  card: UnitCard;
  /** The open faction tab: the look the unit wears (L11). */
  faction: Faction;
}

/** The stage, once the graphics tier is settled; until then the shell holds
 *  its cover over the page. */
export function ArmoryStage(props: StageProps) {
  const { detected } = useMenuShell();
  return detected ? <ReportedStage {...props} /> : null;
}

/** The stage, reporting to the shell's loading cover: ready once it first
 *  draws, or once it has failed, so a failure never holds the cover. */
function ReportedStage({ card, faction }: StageProps) {
  const [ready, setReady] = useState(false);
  const done = useCallback(() => setReady(true), []);
  useLabLoading("renderer", ready);
  return (
    <ArmoryStageBoundary selection={`${card.id}:${faction}`} failed={done}>
      <LiveStage card={card} faction={faction} drawn={done} />
    </ArmoryStageBoundary>
  );
}

/** Every unit switch this page made, for the scene's measure (S7). */
const switches: Switch[] = [];

/** The orbit `spot`'s camera draws `canvas` from: the whole screen, its
 *  centre in the stage's cell (the canvas's parent). */
const stageOrbit = (canvas: HTMLCanvasElement, spot: Spot) =>
  framedOrbit(
    spot.camera,
    canvas.getBoundingClientRect(),
    canvas.parentElement!.getBoundingClientRect(),
  );

const isAbort = (error: unknown) => error instanceof DOMException && error.name === "AbortError";

function LiveStage({ card, faction, drawn }: StageProps & { drawn: () => void }) {
  const catalog = useSessionCatalog();
  const { units } = catalog;
  const audio = useAppAudio();
  const fielding = useStageFielding(catalog);
  // An asynchronous failure is thrown from here, to the stage's boundary.
  const [failure, setFailure] = useState<{ error: unknown } | null>(null);
  if (failure) throw failure.error;
  const fail = useCallback((error: unknown) => setFailure({ error }), []);
  const showground = useShowground(fielding.rules, fail);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const heldRef = useRef<HTMLCanvasElement>(null);
  const [stage, setStage] = useState<ShowcaseStage | null>(null);
  const appearances = useMemo(() => new ArmoryAppearances(units), [units]);
  const shown = useRef<Shown | null>(null);
  const scenery = useRef<ReadonlySet<string> | null>(null);

  // The stage, built once the page's GPU is up, and drawn every frame from
  // the camera of the spot on show.
  useEffect(() => {
    if (!showground) return;
    const canvas = canvasRef.current!;
    let live = true;
    let raf = 0;
    let made: ShowcaseStage | null = null;
    let first = true;
    const camera = (spot: Spot) => {
      const dpr = window.devicePixelRatio || 1;
      const width = Math.max(1, Math.round(canvas.clientWidth * dpr));
      const height = Math.max(1, Math.round(canvas.clientHeight * dpr));
      if (canvas.width !== width || canvas.height !== height)
        [canvas.width, canvas.height] = [width, height];
      return presentedCamera({ camera3d: stageOrbit(canvas, spot), width, height });
    };
    void appResources
      .gpu()
      .then((gpu) => {
        if (!live) return;
        made = ShowcaseStage.create({
          gpu,
          canvas,
          world: yardWorld(),
          models: (installed) => yardModels(yard, units.view.props, installed),
          fielding,
          showground,
          look: STAGE_LOOK,
          failed: fail,
        });
        setStage(made);
        const start = performance.now();
        const loop = (now: number) => {
          if (!live) return;
          const on = shown.current;
          if (on && made!.frame((now - start) / 1000, camera(on.spot)) && first) {
            first = false;
            drawn();
          }
          raf = requestAnimationFrame(loop);
        };
        raf = requestAnimationFrame(loop);
      })
      .catch(fail);
    return () => {
      live = false;
      cancelAnimationFrame(raf);
      made?.dispose();
      setStage(null);
    };
  }, [showground, fielding, units, fail, drawn]);

  // What the stage shows, for the browser scene.
  useEffect(() => {
    if (!stage) return;
    const canvas = canvasRef.current!;
    const orbit = (spot: Spot) => stageOrbit(canvas, spot);
    return publishProbe({ stage, canvas, orbit, audio, switches, shown, appearances });
  }, [stage, audio, appearances]);

  // The page's loops end with the stage, and its art with the page.
  useEffect(
    () => () => {
      audio.showLoops([]);
      appearances.dispose();
    },
    [audio, appearances],
  );

  // Each pick: the unit's art, then the unit on its spot, then its loops.
  const unit = units.has(card.id) ? units.type(card.id) : null;
  useEffect(() => {
    if (!stage) return;
    let live = true;
    const asked = performance.now();
    // A switch rebuilds the stage's scene, and its canvas is clear until the
    // new one draws: the last frame drawn holds behind it meanwhile. The cell
    // says a switch is under way until the next unit stands; a pick during a
    // switch keeps the frame already held, never the cleared canvas.
    const [held, canvas] = [heldRef.current!, canvasRef.current!];
    const cell = canvas.parentElement!;
    if (shown.current && !cell.hasAttribute("data-switching")) {
      [held.width, held.height] = [canvas.width, canvas.height];
      held.getContext("2d")?.drawImage(canvas, 0, 0);
    }
    const switching = (on: boolean) => cell.toggleAttribute("data-switching", on);
    void (async () => {
      if (!unit) {
        // A planned card: the yard alone, from the art already on show.
        audio.showLoops([]);
        const on = shown.current;
        if (!on) return drawn();
        switching(true);
        await stage.show(null, SQUAD_SEED, on.art);
        if (!live) return;
        shown.current = { ...on, kind: null };
        switching(false);
        return;
      }
      switching(true);
      scenery.current ??= yardScenery(yard, units.view.props, await appearances.catalog());
      const art = await appearances.show({ kind: unit.id, faction, scenery: scenery.current });
      if (!live) return;
      const spot = spotFor(yard, unit)!;
      const at: [number, number, number] = [spot.at[0], spot.at[1], 0];
      const loops = await stage.show(
        { kind: unit.id, faction, at, yaw: spot.yaw },
        SQUAD_SEED,
        art,
      );
      if (!live) return;
      shown.current = { kind: unit.id, art, spot };
      switching(false);
      switches.push({ kind: unit.id, ms: performance.now() - asked });
      audio.showLoops(loops);
    })().catch((error: unknown) => {
      // A newer pick superseding this one is a normal switch.
      if (live && !isAbort(error)) fail(error);
    });
    return () => {
      live = false;
    };
  }, [stage, unit, faction, units, appearances, audio, fail, drawn]);

  return (
    <>
      <canvas ref={heldRef} className="armory-stage-canvas" aria-hidden />
      <canvas ref={canvasRef} className="armory-stage-canvas" aria-label="Unit on show" />
    </>
  );
}

interface BoundaryProps {
  /** The pick on show: a new one clears a failure and builds the stage again. */
  selection: string;
  failed: () => void;
  children: ReactNode;
}

/** The failure on show, and the pick it belongs to. */
interface BoundaryState {
  error: unknown;
  selection: string;
}

/** The stage's own failure surface: whatever its subtree throws, drawn on
 *  the stage, never the page's. */
class ArmoryStageBoundary extends Component<BoundaryProps, BoundaryState> {
  state: BoundaryState = { error: null, selection: this.props.selection };

  static getDerivedStateFromError(error: unknown) {
    return { error };
  }

  /** A new pick clears the failure, so the stage is built again for it. */
  static getDerivedStateFromProps(props: BoundaryProps, state: BoundaryState) {
    return props.selection === state.selection ? null : { error: null, selection: props.selection };
  }

  componentDidCatch(error: unknown) {
    console.error("armory stage:", error);
    this.props.failed();
  }

  render() {
    const { error } = this.state;
    if (error === null) return this.props.children;
    return (
      <div className="armory-stage-error" role="alert">
        <p className="armory-stage-error-title">This unit can't be shown</p>
        <p className="armory-stage-error-detail">
          {error instanceof Error ? error.message : String(error)}
        </p>
        <p>Its sheet is still here. Pick another unit, or reload to try again.</p>
      </div>
    );
  }
}
