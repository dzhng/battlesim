import { useState, useMemo, useRef, useEffect, useLayoutEffect } from "react";
import type { PreviewState } from "@web/battle/prepare/preview";
import type { Player } from "@web/battle/prepare/protocol";
import { lanesOf, type LanesModule } from "@web/battle/prepare/lanes";

/** Who holds a side's lane: you, another player, or nobody. */
export interface LaneHolder {
  side: Player["side"];
  lane: number;
  held: "you" | "player" | "none";
}

export function holders(
  wasm: LanesModule,
  sites: string,
  players: readonly Player[],
  you = players.find((player) => player.controller === "human")?.id,
): LaneHolder[] {
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
export function MapFrame({
  state,
  held,
  failureHint,
}: {
  state: PreviewState;
  held: readonly LaneHolder[] | null;
  failureHint?: string;
}) {
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
    for (const tag of frame.querySelectorAll("[data-held]")) {
      tag.removeAttribute("data-held");
      tag.removeAttribute("role");
      tag.removeAttribute("aria-label");
    }
    for (const { side, lane, held: who } of held ?? []) {
      const tag = frame.querySelector(`[data-entry="${side}"][data-lane="${lane}"]`);
      if (!tag) continue;
      tag.setAttribute("data-held", who);
      tag.setAttribute("role", "img");
      tag.setAttribute(
        "aria-label",
        `${side === "blue" ? "Blue" : "Red"} lane ${lane}: ${who === "none" ? "unused" : who === "you" ? "you" : "occupied"}`,
      );
    }
  }, [svg, held]);
  return (
    <div className="lobby-frame" data-status={state.status} data-testid="lobby-map">
      {html && <div ref={picture} className="lobby-picture" dangerouslySetInnerHTML={html} />}
      {state.status !== "ready" && (
        <div className="lobby-frame-state">
          {state.status === "failed" ? (
            <>
              <strong>No map</strong>
              <span>{failureHint ?? "The map could not be made. Reroll for another."}</span>
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
export function PreviewStatus({ state, label }: { state: PreviewState; label?: string }) {
  return (
    <p
      className="lobby-status"
      data-status={state.status}
      data-testid="lobby-status"
      role={label ? "status" : undefined}
      aria-label={label ? "Map preview" : undefined}
    >
      {label ??
        (state.status === "ready"
          ? "Ready"
          : state.status === "retrying"
            ? `Redrawing map · try ${state.attempt}`
            : state.status === "failed"
              ? "No map"
              : "Generating map")}
    </p>
  );
}
