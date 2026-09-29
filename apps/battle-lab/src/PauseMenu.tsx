// The battle's pause menu, in the HUD's look: Esc (one the unit control
// hasn't taken to disarm a command) or the HUD's menu button opens it and
// pauses the battle, and closing it resumes a battle it paused. It holds
// what the player sets between moments of play: the route's own items (the
// scenario, replays), restart, and the sound.
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";
import type { SimClient } from "@web/battle/sim/client";
import { SoundControls } from "./SoundControls";

/** The menu's open state over `client`: opening pauses a running battle and
 *  closing resumes it. */
export function usePauseMenu(client: SimClient | null) {
  const [open, setOpen] = useState(false);
  const resume = useRef(false);
  const show = useCallback(
    (next: boolean) => {
      if (next) {
        resume.current = client?.status === "running";
        if (resume.current) client?.pause();
      } else if (resume.current) {
        resume.current = false;
        client?.resume();
      }
      setOpen(next);
    },
    [client],
  );
  const state = useRef({ open, show });
  state.current = { open, show };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.code !== "Escape" || e.repeat || e.defaultPrevented) return;
      state.current.show(!state.current.open);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  return { open, show };
}

/** The HUD's small button that opens the menu. */
export function MenuButton({ onOpen }: { onOpen: () => void }) {
  return (
    <button
      type="button"
      className="hud-panel hud-menu-button"
      aria-label="Menu"
      title="Menu (Esc)"
      onClick={onOpen}
      data-occludes-readouts
    >
      <Icon path={hudIcon("menu")} />
    </button>
  );
}

export function PauseMenu({
  onClose,
  onRestart,
  children,
}: {
  onClose: () => void;
  /** Restart the battle from its seed; none where a run can't restart. */
  onRestart?: () => void;
  /** The route's own items. */
  children?: ReactNode;
}) {
  return (
    <div className="hud-veil" data-occludes-readouts>
      <div className="hud-panel hud-menu" role="dialog" aria-label="Paused">
        <strong className="hud-menu-title">Paused</strong>
        <button type="button" className="hud-menu-item" onClick={onClose} autoFocus>
          Resume
        </button>
        {onRestart && (
          <button type="button" className="hud-menu-item" onClick={onRestart}>
            Restart
          </button>
        )}
        {children}
        <section className="hud-menu-section" aria-label="Sound">
          <SoundControls />
        </section>
      </div>
    </div>
  );
}
