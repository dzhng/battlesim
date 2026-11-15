// The skirmish lobby, laid out as an order of battle (D22): Blue's player
// banners on the left, Red's on the right, the admitted map between them
// under "N VS M", and a strip below of the match's presets, its map's type,
// size, region and lighting, the preview's status, Reroll and Deploy.
//
// The lobby's rules are the lobby model's (`model.ts`); this view only shows
// them. The map is a real preparation (`mapPreviews`), started again only
// when a map input changes (`isMapInputChange`). Deploy waits for that map,
// then opens the battle the lobby's link names: the admitted seed, so the
// battle is the map the player saw.
import { Link } from "react-router";
import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { FACTIONS } from "@packages/scene-assets/src/units";
import { catalogSet } from "@web/battle/catalog/sets";
import { mapPreviews, type MapPreviews, type PreviewState } from "@web/battle/prepare/preview";
import type { Player } from "@web/battle/prepare/protocol";
import { lanesOf, type LanesModule } from "@web/battle/prepare/lanes";
import { loadWasm } from "@web/battle/sim/module";
import { MAP_SIZES, MAP_TYPES, newSeed } from "@web/maps/source";
import { REGIONS, spoken, type AskedLobby } from "../battleLinks";
import { MAX_PLAYERS_PER_SIDE } from "../skirmishSetup";
import {
  add,
  isMapInputChange,
  lobbyCandidate,
  preset,
  remove,
  reroll,
  seatHuman,
  setFaction,
  setLighting,
  setRegion,
  setSize,
  setType,
  sideFaction,
  toUrl,
  type LobbyPreset,
  type LobbySide,
  type LobbyState,
} from "./model";
import { lightingChoices, lightingLabel } from "./lighting";

const RANDOM = "random";
const PRESETS: readonly LobbyPreset[] = ["1v1", "3v3", "5v5"];

/** A 1v1 on the asked map, at the asked seed or a fresh one. */
function opening(asked: AskedLobby): LobbyState {
  let state = preset("1v1");
  if (asked.faction) state = setFaction(state, "blue", asked.faction);
  if (asked.type) state = setType(state, asked.type);
  if (asked.size) state = setSize(state, asked.size);
  if (asked.lighting) state = setLighting(state, asked.lighting);
  return reroll(setRegion(state, asked.region), asked.seed ?? newSeed());
}

/** The lobby and its map preview. `update` takes the lobby's next state and
 *  starts a new preview at once when a map input changed, so the screen
 *  never shows an older map as the one Deploy would start. A preview that
 *  had to draw another seed hands that seed to the lobby without starting
 *  again. */
function useLobby(asked: AskedLobby) {
  const [lobby, setLobby] = useState(() => opening(asked));
  const [preview, setPreview] = useState<PreviewState>({ status: "loading" });
  const current = useRef(lobby);
  /** The lobby state the running preview shows. */
  const shown = useRef<LobbyState | null>(null);
  const previews = useRef<{ show(state: LobbyState): void } | null>(null);
  const [wasm, setWasm] = useState<LanesModule | null>(null);

  const update = (next: LobbyState) => {
    current.current = next;
    setLobby(next);
    if (shown.current && !isMapInputChange(shown.current, next)) return;
    shown.current = null;
    previews.current?.show(next);
  };

  useEffect(() => {
    let live = true;
    let controller: MapPreviews | null = null;
    // Each preview tries the lobby's own seed first; a hidden retry draws a
    // fresh one.
    let firstSeed: string | null = null;
    const adopt = (state: PreviewState) => {
      setPreview(state);
      if (state.status !== "ready" || !shown.current) return;
      const seed = state.preview.request.map_source.request.seed;
      if (seed === shown.current.map.seed) return;
      const adopted = reroll(current.current, seed);
      shown.current = adopted;
      current.current = adopted;
      setLobby(adopted);
    };
    void Promise.all([loadWasm(), catalogSet("game")]).then(
      ([wasm, catalog]) => {
        if (!live) return;
        setWasm(wasm);
        controller = mapPreviews(
          {
            documents: { rules: JSON.stringify(catalog.rules), presets, templates },
            policy: config.admission,
            onChange: (state) => live && adopt(state),
          },
          {
            seed: () => {
              const seed = firstSeed ?? newSeed();
              firstSeed = null;
              return seed;
            },
          },
        );
        const running = controller;
        previews.current = {
          show(state) {
            shown.current = state;
            firstSeed = state.map.seed;
            running.show(lobbyCandidate(wasm, state));
          },
        };
        previews.current.show(current.current);
      },
      (error: unknown) =>
        live &&
        setPreview({
          status: "failed",
          message: error instanceof Error ? error.message : String(error),
          attempts: [],
        }),
    );
    return () => {
      live = false;
      controller?.dispose();
      previews.current = null;
      shown.current = null;
    };
  }, []);

  return { lobby, preview, update, wasm };
}

/** The skirmish page: set up any match and deploy it. */
export function Lobby({ asked }: { asked: AskedLobby }) {
  const { lobby, preview, update, wasm } = useLobby(asked);
  const count = (side: LobbySide) => lobby.players.filter((p) => p.side === side).length;
  const ready = preview.status === "ready";
  const sites = ready ? preview.preview.sites : null;
  const held = useMemo(
    () => (wasm && sites ? holders(wasm, sites, lobby.players) : null),
    [wasm, sites, lobby.players],
  );
  return (
    <section className="lobby" aria-label="New battle">
      <div className="lobby-field">
        <SideColumn side="blue" lobby={lobby} update={update} />
        <figure className="lobby-map">
          <figcaption className="lobby-vs" data-testid="lobby-vs">
            <span data-side="blue">{count("blue")}</span> VS{" "}
            <span data-side="red">{count("red")}</span>
          </figcaption>
          <MapFrame state={preview} held={held} />
        </figure>
        <SideColumn side="red" lobby={lobby} update={update} />
      </div>
      <div className="lobby-strip">
        <Choice
          label="match"
          options={PRESETS}
          value={PRESETS.find((kind) => kind === `${count("blue")}v${count("red")}`) ?? null}
          onChange={(kind) => update(preset(kind, lobby))}
        />
        <Choice
          label="map"
          options={MAP_TYPES}
          value={lobby.map.type}
          onChange={(type) => update(setType(lobby, type))}
        />
        <Choice
          label="size"
          options={MAP_SIZES}
          value={lobby.map.size}
          onChange={(size) => update(setSize(lobby, size))}
          // Sizes too small for the larger side are closed (D18).
          closed={(size) => MAP_SIZES.indexOf(size) < MAP_SIZES.indexOf(lobby.sizeFloor)}
        />
        <Choice
          label="region"
          folds
          options={[RANDOM, ...REGIONS]}
          value={lobby.map.region ?? RANDOM}
          onChange={(region) => update(setRegion(lobby, region === RANDOM ? undefined : region))}
        />
        <Choice
          label="lighting"
          folds
          options={lightingChoices()}
          value={lobby.lighting}
          onChange={(lighting) => update(setLighting(lobby, lighting))}
          say={lightingLabel}
        />
        <PreviewStatus state={preview} />
        <button
          type="button"
          className="lobby-reroll"
          data-testid="lobby-reroll"
          onClick={() => update(reroll(lobby))}
        >
          Reroll
        </button>
        {ready ? (
          <Link
            className="lobby-deploy"
            data-testid="menu-deploy"
            target="_self"
            to={`/battle${toUrl(lobby)}`}
          >
            Deploy
          </Link>
        ) : (
          <a className="lobby-deploy" data-testid="menu-deploy" role="link" aria-disabled="true">
            Deploy
          </a>
        )}
      </div>
    </section>
  );
}

/** One side's order of battle: its faction, then a banner per player, then
 *  room for another. */
function SideColumn({
  side,
  lobby,
  update,
}: {
  side: LobbySide;
  lobby: LobbyState;
  update: (next: LobbyState) => void;
}) {
  const players = lobby.players.filter((player) => player.side === side);
  const name = side === "blue" ? "Blue" : "Red";
  return (
    <div className="lobby-side" data-side={side} aria-label={`${name} side`} role="group">
      <Pick
        label={`${name} faction`}
        testId={`lobby-faction-${side}`}
        className="lobby-faction"
        options={FACTIONS}
        value={sideFaction(lobby, side)}
        onChange={(faction) => update(setFaction(lobby, side, faction))}
      >
        {name} · {spoken(sideFaction(lobby, side))}
      </Pick>
      <ol className="lobby-banners">
        {players.map((player, index) => (
          <Banner
            key={player.id}
            player={player}
            index={index + 1}
            removable={players.length > 1 && player.controller !== "human"}
            onSeat={() => update(seatHuman(lobby, player))}
            onRemove={() => update(remove(lobby, player))}
          />
        ))}
      </ol>
      {players.length < MAX_PLAYERS_PER_SIDE && (
        <button
          type="button"
          className="lobby-add"
          data-testid={`lobby-add-${side}`}
          onClick={() => update(add(lobby, side))}
        >
          Add player
        </button>
      )}
    </div>
  );
}

/** A player's banner: its number on its side, who commands it, and the
 *  delete mark. A Blue AI banner can be taken: the player moves there. */
function Banner({
  player,
  index,
  removable,
  onSeat,
  onRemove,
}: {
  player: Player;
  index: number;
  removable: boolean;
  onSeat: () => void;
  onRemove: () => void;
}) {
  const you = player.controller === "human";
  const name = `${player.side === "blue" ? "Blue" : "Red"} ${index}`;
  return (
    <li className="lobby-banner" data-player={player.id} data-you={you || undefined}>
      <span className="lobby-index" aria-hidden>
        {index}
      </span>
      {player.side === "blue" && !you ? (
        <Pick
          label={`${name} controller`}
          testId={`lobby-controller-${player.id}`}
          className="lobby-controller"
          options={["ai", "you"] as const}
          value="ai"
          onChange={(choice) => choice === "you" && onSeat()}
        >
          AI
        </Pick>
      ) : (
        <span className="lobby-controller" data-testid={`lobby-controller-${player.id}`}>
          {you ? "You" : "AI"}
        </span>
      )}
      {removable && (
        <button
          type="button"
          className="lobby-remove"
          aria-label={`Remove ${name}`}
          data-testid={`lobby-remove-${player.id}`}
          onClick={onRemove}
        />
      )}
    </li>
  );
}

/** A value picked from a short list that opens under it, closing on a pick,
 *  on Escape or when focus leaves it. */
function Pick<T extends string>({
  label,
  testId,
  className,
  options,
  value,
  onChange,
  say = spoken,
  children,
}: {
  label: string;
  testId: string;
  className: string;
  options: readonly T[];
  value: T;
  onChange: (value: T) => void;
  say?: (option: T) => string;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const close = (event: KeyboardEvent) => {
    if (event.key !== "Escape" || !open) return;
    // Escape closes the list, not the page.
    event.stopPropagation();
    setOpen(false);
  };
  return (
    <span
      className={`lobby-pick ${className}`}
      data-open={open || undefined}
      onKeyDown={close}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
      }}
    >
      <button
        type="button"
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        data-testid={testId}
        onClick={() => setOpen(!open)}
      >
        {children}
      </button>
      {open && (
        <ul role="listbox" aria-label={label}>
          {options.map((option) => (
            <li key={option}>
              <button
                type="button"
                role="option"
                aria-selected={option === value}
                data-testid={`${testId}-${option}`}
                onClick={() => {
                  setOpen(false);
                  if (option !== value) onChange(option);
                }}
              >
                {say(option)}
              </button>
            </li>
          ))}
        </ul>
      )}
    </span>
  );
}

/** Windows narrower than this fold the strip's longest choices into
 *  pickers, so the strip stays one row down to the smallest window the game
 *  supports (1280 wide: web/README.md). */
const FOLDED_STRIP = "(max-width: 1499px)";
const stripFolds = () => globalThis.matchMedia?.(FOLDED_STRIP).matches ?? false;
const watchFold = (change: () => void) => {
  const query = globalThis.matchMedia?.(FOLDED_STRIP);
  query?.addEventListener("change", change);
  return () => query?.removeEventListener("change", change);
};

/** One of a few named options, side by side. An option can be closed. One
 *  that `folds` is a picker of its options in a narrow window. */
function Choice<T extends string>({
  label,
  options,
  value,
  onChange,
  closed = () => false,
  say = spoken,
  folds = false,
}: {
  label: string;
  options: readonly T[];
  value: T | null;
  onChange: (value: T) => void;
  closed?: (option: T) => boolean;
  say?: (option: T) => string;
  folds?: boolean;
}) {
  const folded = useSyncExternalStore(watchFold, stripFolds) && folds && value !== null;
  return (
    <div className="lobby-choice">
      <span className="lobby-label" id={`menu-${label}`}>
        {label}
      </span>
      {folded ? (
        <Pick
          label={label}
          testId={`menu-${label}`}
          className="lobby-folded"
          options={options.filter((option) => !closed(option))}
          value={value}
          onChange={onChange}
          say={say}
        >
          {say(value)}
        </Pick>
      ) : (
        <div role="radiogroup" aria-labelledby={`menu-${label}`} className="lobby-options">
          {options.map((option) => (
            <button
              key={option}
              type="button"
              role="radio"
              aria-checked={option === value}
              disabled={closed(option)}
              data-testid={`menu-${label}-${option}`}
              onClick={() => option !== value && onChange(option)}
            >
              {say(option)}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/** Who holds a side's lane: you, another player, or nobody. */
interface LaneHolder {
  side: Player["side"];
  lane: number;
  held: "you" | "player" | "none";
}

function holders(wasm: LanesModule, sites: string, players: readonly Player[]): LaneHolder[] {
  const you = players.find((player) => player.controller === "human")?.id;
  return lanesOf(wasm, sites, players).entries.map(({ side, lane, holder }) => ({
    side,
    lane,
    held: holder === null ? "none" : holder === you ? "you" : "player",
  }));
}

/** The map frame: the admitted map once it is ready, and until then what its
 *  preparation is doing over the last map shown. The picture holds no
 *  players, so each entry tag (found by its side and lane) is marked with who
 *  holds that lane, and the stylesheet draws an open lane and yours. A seat
 *  change re-marks the tags without drawing a new map. */
function MapFrame({ state, held }: { state: PreviewState; held: readonly LaneHolder[] | null }) {
  const [last, setLast] = useState<string | null>(null);
  const picture = useRef<HTMLDivElement>(null);
  const svg = state.status === "ready" ? state.preview.svg : last;
  // One markup object per picture: React sets the inner HTML again whenever
  // this object is a new one, which would re-parse the picture and drop the
  // tags' marks on every render.
  const html = useMemo(() => (svg ? { __html: svg } : null), [svg]);
  useEffect(() => {
    if (state.status === "ready") setLast(state.preview.svg);
  }, [state]);
  useLayoutEffect(() => {
    const frame = picture.current;
    if (!frame) return;
    for (const tag of frame.querySelectorAll("[data-held]")) tag.removeAttribute("data-held");
    for (const { side, lane, held: who } of held ?? [])
      frame
        .querySelector(`[data-entry="${side}"][data-lane="${lane}"]`)
        ?.setAttribute("data-held", who);
  }, [svg, held]);
  return (
    <div className="lobby-frame" data-status={state.status} data-testid="lobby-map">
      {html && <div ref={picture} className="lobby-picture" dangerouslySetInnerHTML={html} />}
      {state.status !== "ready" && (
        <div className="lobby-frame-state">
          {state.status === "failed" ? (
            <>
              <strong>No map</strong>
              <span>The map could not be made. Reroll for another.</span>
            </>
          ) : (
            <strong>{state.status === "retrying" ? "Redrawing map" : "Generating map"}</strong>
          )}
        </div>
      )}
    </div>
  );
}

/** Quiet word on the map: whether it is coming, being redrawn, ready, or
 *  missing. */
function PreviewStatus({ state }: { state: PreviewState }) {
  return (
    <p className="lobby-status" data-status={state.status} data-testid="lobby-status">
      {state.status === "ready"
        ? "Ready"
        : state.status === "retrying"
          ? `Redrawing map · try ${state.attempt}`
          : state.status === "failed"
            ? "No map"
            : "Generating map"}
    </p>
  );
}
