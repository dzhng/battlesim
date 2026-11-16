import { useCallback, useEffect, useRef, useState } from "react";
import type { BattleTicket, Briefing, RoomList } from "@packages/multiplayer/src/wire";
import { StreamRefused } from "@packages/multiplayer/src/stream/http";
import { useLocation, useNavigate } from "react-router";
import { SessionCatalogScope } from "@web/battle/catalog/context";
import { followBattle } from "../onlineSession";
import { BriefingCard } from "./BriefingCard";
import type { OnlineMap } from "./useOnlineMap";
import { leaveFrontline } from "./frontlineRequests";
import { hello } from "./hello";
import { roomFailure, onlineRequest } from "./requests";
import "./frontline.css";

export function Frontline({
  address,
  counts,
  onCallsign,
}: {
  address: string;
  counts: RoomList["frontline"] | undefined;
  onCallsign: (name: string) => void;
}) {
  const navigate = useNavigate();
  const location = useLocation();
  const [briefing, setBriefing] = useState<Briefing | null>(null);
  const held = useRef<string | null>(null);
  const live = useRef(false);
  const leave = useCallback((battle: string) => leaveFrontline(address, battle), [address]);
  useEffect(() => {
    live.current = true;
    return () => {
      live.current = false;
      if (held.current) void leave(held.current).catch(() => {});
    };
    // The address is fixed for this page visit.
  }, [leave]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(
    () => (location.state as { frontlineNotice?: string } | null)?.frontlineNotice ?? null,
  );
  const pending = useRef(false);
  const play = async () => {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setNotice(null);
    const visit = new AbortController();
    try {
      const join = () => onlineRequest<Briefing>(address, "/frontline/join", {}, visit.signal);
      let briefing: Briefing;
      try {
        briefing = await join();
      } catch (error) {
        if (!(error instanceof StreamRefused) || error.reason !== "callsign") throw error;
        const assigned = await hello(address, visit.signal);
        if (live.current) onCallsign(assigned);
        briefing = await join();
      }
      if (!live.current) {
        void leave(briefing.battle).catch(() => {});
        return;
      }
      held.current = briefing.battle;
      setBriefing(briefing);
    } catch (error) {
      if (!live.current) return;
      setNotice(
        error instanceof StreamRefused && error.reason === "unavailable"
          ? "No battlefield could be made. Try again."
          : error instanceof StreamRefused &&
              (error.reason === "full" || error.reason === "game_running")
            ? roomFailure(error)
            : "Could not join Frontline. Try again.",
      );
    } finally {
      pending.current = false;
      if (live.current) setBusy(false);
    }
  };
  const back = async () => {
    if (!briefing || pending.current) return;
    pending.current = true;
    setBusy(true);
    try {
      await leave(briefing.battle);
      held.current = null;
      if (live.current) {
        setBriefing(null);
        setNotice(null);
      }
    } catch {
      if (live.current) setNotice("Could not return the seat. Try again.");
    } finally {
      pending.current = false;
      if (live.current) setBusy(false);
    }
  };
  const deploy = async (prepared: OnlineMap) => {
    if (!briefing || pending.current) return;
    pending.current = true;
    setBusy(true);
    setNotice(null);
    try {
      const ticket = await onlineRequest<BattleTicket>(
        address,
        `/frontline/${encodeURIComponent(briefing.battle)}/deploy`,
        {},
        new AbortController().signal,
      );
      if (!live.current) {
        void leave(ticket.battle).catch(() => {});
        return;
      }
      held.current = null;
      followBattle(navigate, { ticket, address }, location.search, prepared);
    } catch (error) {
      if (!live.current) return;
      if (error instanceof StreamRefused && error.reason === "expired") {
        held.current = null;
        setBriefing(null);
        setNotice("Your seat was given away. Press Play to find another command.");
      } else setNotice("Could not deploy. Try again.");
    } finally {
      pending.current = false;
      if (live.current) setBusy(false);
    }
  };
  useEffect(() => {
    if (!briefing) return;
    const escape = (event: KeyboardEvent) => {
      if (event.code !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      void back();
    };
    window.addEventListener("keydown", escape, true);
    return () => window.removeEventListener("keydown", escape, true);
  });
  if (briefing)
    return (
      <SessionCatalogScope set="game">
        <BriefingCard
          briefing={briefing}
          busy={busy}
          error={notice}
          onBack={() => void back()}
          onDeploy={(prepared) => void deploy(prepared)}
        />
      </SessionCatalogScope>
    );
  return (
    <section className="online-frontline" aria-label="Frontline">
      <div>
        <h3>Frontline</h3>
        <p>Join a rolling battle on the side that needs a commander.</p>
        <p role="status" aria-label="Frontline activity">
          {counts
            ? `${counts.battles} ${counts.battles === 1 ? "battle" : "battles"} under way · ${counts.commanders} ${counts.commanders === 1 ? "commander" : "commanders"}`
            : "Finding battles…"}
        </p>
        {notice && <p role="alert">{notice}</p>}
      </div>
      <button type="button" className="online-start" disabled={busy} onClick={() => void play()}>
        {busy ? "Finding a battle…" : "Play"}
      </button>
    </section>
  );
}
