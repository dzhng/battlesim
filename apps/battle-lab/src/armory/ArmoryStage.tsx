// The Armory's stage: the open card's unit standing on its spot in the yard,
// with the battle's idle life and idle sound, seen from the spot's camera.
// It is one `ShowcaseStage` for the page's life; each pick asks
// `ArmoryAppearances` for that unit's art and shows it. A planned card asks
// for no unit: the sheet carries it, and the yard stands empty. A failure
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
import { ArmoryAppearances } from "./armoryAppearances";
import { ArmoryCamera, useArmoryCamera } from "./armoryCamera";
import { framingOf } from "./stageFraming";
import { publishProbe, type Shown, type Switch } from "./armoryProbe";
import { STAGE_LOOK, SQUAD_SEED, useShowground, useStageFielding } from "./stageInputs";
import { spotFor, yard, yardModels, yardScenery, yardWorld, yardWorldScenery } from "./yard";

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
  const camera = useMemo(() => new ArmoryCamera(), []);
  useArmoryCamera(canvasRef, camera);

  // The stage, built once the page's GPU is up, and drawn every frame from
  // the camera of the spot on show.
  useEffect(() => {
    if (!showground) return;
    const canvas = canvasRef.current!;
    let live = true;
    let raf = 0;
    let made: ShowcaseStage | null = null;
    let first = true;
    let last = performance.now();
    void appResources
      .gpu()
      .then((gpu) => {
        if (!live) return;
        made = new ShowcaseStage({
          gpu,
          canvas,
          world: yardWorld(),
          models: (installed) => yardModels(yard, units.view.props, installed),
          scenery: yardWorldScenery,
          fielding,
          showground,
          look: STAGE_LOOK,
          failed: fail,
        });
        setStage(made);
        const start = performance.now();
        const loop = (now: number) => {
          if (!live) return;
          const dt = Math.min(0.1, (now - last) / 1000);
          last = now;
          // The camera for this frame: the whole canvas, the unit in the cell
          // (none once the canvas has left the page).
          const presented = (width: number, height: number) => {
            const cell = canvas.parentElement?.getBoundingClientRect();
            return cell
              ? camera.presented(canvas.getBoundingClientRect(), cell, width, height, dt)
              : null;
          };
          if (made!.frame((now - start) / 1000, presented) && first) {
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
  }, [showground, fielding, units, fail, drawn, camera]);

  // What the stage shows, for the browser scene.
  useEffect(() => {
    if (!stage) return;
    const canvas = canvasRef.current!;
    return publishProbe({ stage, canvas, camera, audio, switches, shown, appearances });
  }, [stage, audio, appearances, camera]);

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
      switching(true);
      scenery.current ??= yardScenery(yard, units.view.props, await appearances.catalog());
      if (!unit) {
        // A planned card: the yard alone, from the art already on show, or
        // the yard's own art on a first open, seen from the motor pool.
        audio.showLoops([]);
        const on = shown.current ?? {
          art: await appearances.show({ kind: null, faction, scenery: scenery.current }),
          spot: { id: yard.bodies.ground, ...yard.spots[yard.bodies.ground] },
        };
        if (!live) return;
        await stage.show(null, SQUAD_SEED, on.art);
        if (!live) return;
        shown.current = { ...on, kind: null };
        camera.frame(framingOf(stage, on.art, units, catalog.rules, on.spot));
        switching(false);
        return;
      }
      const art = await appearances.show({ kind: unit.id, faction, scenery: scenery.current });
      if (!live) return;
      const spot = spotFor(yard, unit);
      const at: [number, number, number] = [spot.at[0], spot.at[1], 0];
      const loops = await stage.show(
        { kind: unit.id, faction, at, yaw: spot.yaw },
        SQUAD_SEED,
        art,
      );
      if (!live) return;
      shown.current = { kind: unit.id, art, spot };
      camera.frame(framingOf(stage, art, units, catalog.rules, spot));
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
  }, [stage, unit, faction, units, catalog, appearances, audio, fail, camera]);

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
        <p>Its sheet is still here. Pick another unit, or reload to try again.</p>
        <details className="armory-stage-error-detail">
          <summary>Details</summary>
          {error instanceof Error ? error.message : String(error)}
        </details>
      </div>
    );
  }
}
