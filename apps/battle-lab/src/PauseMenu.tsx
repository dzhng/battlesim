import { Link } from "react-router";
// The battle's pause menu, in the HUD's look: Esc (one the unit control
// hasn't taken to disarm a command) or the HUD's menu button opens it and
// pauses the battle, and closing it resumes a battle it paused. It holds
// what the player sets between moments of play: the route's own items (the
// scenario, replays), restart, and the sound.
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";
import type { SimClient } from "@web/battle/sim/client";
import { DiagnosticsControls } from "./DiagnosticsControls";
import { GraphicsSettingsControls } from "./GraphicsSettingsControls";
import { SoundControls } from "./SoundControls";
import { Tutorial } from "./Tutorial";

/** The menu's open state over `client`: opening pauses an unpaused battle and
 *  closing resumes only a pause it owns. */
export function usePauseMenu(
  client: SimClient | null,
  open: boolean,
  setOpen: (open: boolean) => void,
) {
  const resume = useRef(false);
  const show = useCallback(
    (next: boolean) => {
      if (next && !resume.current) {
        resume.current = !!client && !client.paused;
        if (resume.current) client?.pause();
      } else if (!next && resume.current) {
        resume.current = false;
        client?.resume();
      }
      setOpen(next);
    },
    [client, setOpen],
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
  onSettingsApply,
  children,
}: {
  onClose: () => void;
  /** Restart the battle from its seed; none where a run can't restart. */
  onRestart?: () => void;
  onSettingsApply?: () => void | Promise<void>;
  /** The route's own items. */
  children?: ReactNode;
}) {
  const [tutorialOpen, setTutorialOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  useEffect(() => {
    if (!tutorialOpen && !settingsOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.code !== "Escape" || e.repeat || e.defaultPrevented) return;
      e.preventDefault();
      e.stopPropagation();
      setTutorialOpen(false);
      setSettingsOpen(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [tutorialOpen, settingsOpen]);
  return (
    <>
      <div className="hud-veil" data-occludes-readouts>
        <div
          className="hud-panel hud-menu"
          role="dialog"
          aria-modal="true"
          aria-label="Paused"
          aria-hidden={tutorialOpen || settingsOpen || undefined}
          inert={tutorialOpen || settingsOpen || undefined}
        >
          <strong className="hud-menu-title">Paused</strong>
          <button type="button" className="hud-menu-item" onClick={onClose} autoFocus>
            Resume
          </button>
          <button type="button" className="hud-menu-item" onClick={() => setTutorialOpen(true)}>
            Tutorial
          </button>
          <button type="button" className="hud-menu-item" onClick={() => setSettingsOpen(true)}>
            Settings
          </button>
          {onRestart && (
            <button type="button" className="hud-menu-item" onClick={onRestart}>
              Restart
            </button>
          )}
          {children}
          <Link className="hud-menu-item" to="/">
            Main menu
          </Link>
        </div>
      </div>
      {tutorialOpen && (
        <div className="hud-veil hud-tutorial-veil" data-occludes-readouts>
          <div
            className="hud-panel hud-tutorial-modal"
            role="dialog"
            aria-modal="true"
            aria-label="Tutorial"
          >
            <header className="hud-tutorial-head">
              <strong className="hud-menu-title">Tutorial</strong>
              <button
                type="button"
                className="hud-menu-item"
                onClick={() => setTutorialOpen(false)}
                autoFocus
              >
                Close tutorial
              </button>
            </header>
            <Tutorial />
          </div>
        </div>
      )}
      {settingsOpen && (
        <div className="hud-veil hud-tutorial-veil" data-occludes-readouts>
          <div
            className="hud-panel hud-tutorial-modal"
            role="dialog"
            aria-modal="true"
            aria-label="Settings"
          >
            <header className="hud-tutorial-head">
              <strong className="hud-menu-title">Settings</strong>
              <button
                type="button"
                className="hud-menu-item"
                onClick={() => setSettingsOpen(false)}
                autoFocus
              >
                Close settings
              </button>
            </header>
            <GraphicsSettingsControls onApply={onSettingsApply} />
            <section className="hud-menu-section" aria-label="Sound">
              <SoundControls />
            </section>
            <section className="hud-menu-section" aria-label="Diagnostics">
              <DiagnosticsControls />
            </section>
          </div>
        </div>
      )}
    </>
  );
}
