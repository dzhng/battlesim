import { newSeed } from "@web/maps/source";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router";
import type { RoomAction, RoomView } from "@packages/multiplayer/src/wire";
import type { LobbyDriver } from "../lobby/driver";
import { serverAddress } from "./server";
import { hello } from "./hello";
import { stateStream } from "./stateStream";
import { roomFailure, roomRequest } from "./roomRequests";
import { roomLobby, useRoomPreview } from "./roomPreview";

export function useRoom(code: string) {
  const navigate = useNavigate();
  const location = useLocation();
  const [address] = useState(() => serverAddress(window.location));
  const [view, setView] = useState<RoomView | null>(null);
  const [unavailable, setUnavailable] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const lifetime = useRef<AbortController | null>(null);
  const leavePage = useCallback(
    (notice?: string) => {
      navigate(`/multiplayer${location.search}`, { replace: true, state: { roomNotice: notice } });
    },
    [navigate, location.search],
  );
  useEffect(() => {
    const visit = new AbortController();
    lifetime.current = visit;
    if (!address) {
      setUnavailable(true);
      return () => visit.abort();
    }
    void hello(address, visit.signal)
      .then(() => {
        if (visit.signal.aborted) return;
        stateStream<RoomView>(
          address,
          `/rooms/${encodeURIComponent(code)}`,
          visit.signal,
          (next) => {
            if (!next.you) {
              leavePage("You are no longer in that room.");
              return;
            }
            setView(next);
          },
          () => leavePage("That room has closed."),
          (error) => leavePage(roomFailure(error)),
        );
      })
      .catch(() => {
        if (!visit.signal.aborted) setUnavailable(true);
      });
    return () => visit.abort();
  }, [address, code, leavePage]);
  const act = useCallback(
    (action: RoomAction) => {
      const signal = lifetime.current!.signal;
      setError(null);
      void roomRequest(address, `/rooms/${encodeURIComponent(code)}/act`, action, signal)
        .then(() => {
          if (!signal.aborted && action.kind === "leave") leavePage();
        })
        .catch((error: unknown) => {
          if (!signal.aborted) setError(roomFailure(error));
        });
    },
    [address, code, leavePage],
  );
  return { view, unavailable, error, act };
}

export function useRoomDriver(view: RoomView, act: (action: RoomAction) => void): LobbyDriver {
  const lobby = useMemo(() => roomLobby(view), [view]);
  const preview = useRoomPreview(lobby);
  const state = preview.preview;
  const own = view.seats.find((seat) => seat.id === view.you)?.occupant;
  return {
    kind: "room",
    previewStatus:
      state.status === "ready"
        ? "Map ready"
        : state.status === "failed"
          ? "Map unavailable"
          : state.status === "retrying"
            ? `Map redrawing · try ${state.attempt}`
            : "Map generating",
    lobby,
    view,
    setType: (type) => act({ kind: "settings", patch: { map: { type } } }),
    setSize: (size) => act({ kind: "settings", patch: { map: { size } } }),
    setRegion: (region) => act({ kind: "settings", patch: { map: { region: region ?? null } } }),
    setLighting: (lighting) => act({ kind: "settings", patch: { lighting } }),
    setFaction: (side, faction) =>
      act({ kind: "settings", patch: { factions: { [side]: faction } } }),
    reroll: () => act({ kind: "settings", patch: { map: { seed: newSeed() } } }),
    sit: (seat) => act({ kind: "sit", seat }),
    remove: (seat) => act({ kind: "remove_seat", seat }),
    add: (side) => act({ kind: "add_seat", side }),
    setSeat: (seat, to) => act({ kind: "set_seat", seat, to }),
    ready: (ready) => act({ kind: "ready", ready }),
    start: () => act({ kind: "start" }),
    editable: own?.kind === "human" && own.host,
    ...preview,
  };
}
