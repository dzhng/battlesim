import { Link } from "react-router";
// The battle's pause menu, in the HUD's look: Esc (one the unit control
// hasn't taken to disarm a command) or the HUD's menu button opens it and
// pauses the battle, and closing it resumes a battle it paused. It holds
// what the player reaches between moments of play: the Guide, the settings,
// full screen, restart, and the route's own items (the scenario). Restart and
// main menu ask first: either loses the battle in progress. Leaving an online battle
// loses nothing (the seat's AI takes over, and the seat can be returned to),
// so its item leaves at once.
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";
import type { SimClient } from "@web/battle/sim/client";
import { DiagnosticsControls } from "./DiagnosticsControls";
import { GraphicsSettingsControls } from "./GraphicsSettingsControls";
import { SoundControls } from "./SoundControls";
import { Guide } from "./Guide";
import { fullscreenLabel, toggleFullscreen, useFullscreen } from "./fullscreen";
import { FullscreenBinding } from "@web/battle/input/commandBindings";
import { SaveReplayButton } from "./replayFile";

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
      className="hud-panel hud-icon-button hud-menu-button"
      aria-label="Menu"
      title="Menu (Esc)"
      onClick={onOpen}
      data-occludes-readouts
    >
      <Icon path={hudIcon("menu")} />
    </button>
  );
}

/** What the menu asks before an item that loses the battle in progress. */
const questions = {
  leave: { title: "Leave the battle?", back: "Stay", go: "Leave battle" },
  restart: { title: "Restart the battle?", back: "Cancel", go: "Restart battle" },
  chapter: { title: "Restart the chapter?", back: "Cancel", go: "Restart chapter" },
};

export function PauseMenu({
  onClose,
  online = false,
  onRestart,
  onRestartChapter,
  onLeave,
  onSettingsApply,
  onSaveReplay,
  children,
}: {
  onClose: () => void;
  online?: boolean;
  /** Restart the battle from its seed; none where a run can't restart. */
  onRestart?: () => void;
  /** Take an authored scenario back to its open chapter's start; none
   *  before that start is ready to restart. */
  onRestartChapter?: () => void;
  /** Leave an online battle for Multiplayer; none offline, which asks first
   *  and goes to the main menu. */
  onLeave?: () => void;
  onSettingsApply?: () => void | Promise<void>;
  onSaveReplay?: () => Promise<unknown>;
  /** The route's own items. */
  children?: ReactNode;
}) {
  const [guideOpen, setGuideOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [asking, setAsking] = useState<keyof typeof questions | null>(null);
  const fullscreen = fullscreenLabel(useFullscreen());
  const question = asking && questions[asking];
  // Escape closes what is open above the menu, or backs out of a question.
  useEffect(() => {
    if (!guideOpen && !settingsOpen && !asking) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.code !== "Escape" || e.repeat || e.defaultPrevented) return;
      e.preventDefault();
      e.stopPropagation();
      setGuideOpen(false);
      setSettingsOpen(false);
      setAsking(null);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [guideOpen, settingsOpen, asking]);
  return (
    <>
      <div className="hud-veil" data-occludes-readouts>
        <div
          className="hud-panel hud-menu"
          role="dialog"
          aria-modal="true"
          aria-label={question ? question.title : online ? "Menu" : "Paused"}
          aria-hidden={guideOpen || settingsOpen || undefined}
          inert={guideOpen || settingsOpen || undefined}
        >
          {question ? (
            <>
              <strong className="hud-menu-title">{question.title}</strong>
              <div className="hud-menu-note">
                {asking === "chapter"
                  ? "Progress since the chapter began will be lost."
                  : "Progress in this battle will be lost."}
              </div>
              <button
                type="button"
                className="hud-menu-item"
                onClick={() => setAsking(null)}
                autoFocus
              >
                {question.back}
              </button>
              {asking === "leave" ? (
                <Link className="hud-menu-item" to="/">
                  {question.go}
                </Link>
              ) : (
                <button
                  type="button"
                  className="hud-menu-item"
                  onClick={asking === "chapter" ? onRestartChapter : onRestart}
                >
                  {question.go}
                </button>
              )}
            </>
          ) : (
            <>
              <strong className="hud-menu-title">{online ? "Menu" : "Paused"}</strong>
              <button type="button" className="hud-menu-item" onClick={onClose} autoFocus>
                {online ? "Back to battle" : "Resume"}
              </button>
              <button
                type="button"
                className="hud-menu-item"
                data-menu-item="guide"
                onClick={() => setGuideOpen(true)}
              >
                Guide
              </button>
              <button type="button" className="hud-menu-item" onClick={() => setSettingsOpen(true)}>
                Settings
              </button>
              <button
                type="button"
                className="hud-menu-item"
                title={`${fullscreen} (${FullscreenBinding.label})`}
                onClick={() => void toggleFullscreen().catch(() => {})}
              >
                {fullscreen}
              </button>
              {onRestart && (
                <button
                  type="button"
                  className="hud-menu-item"
                  onClick={() => setAsking("restart")}
                >
                  Restart
                </button>
              )}
              {onRestartChapter && (
                <button
                  type="button"
                  className="hud-menu-item"
                  onClick={() => setAsking("chapter")}
                >
                  Restart chapter
                </button>
              )}
              {children}
              {onLeave ? (
                <button type="button" className="hud-menu-item" onClick={onLeave}>
                  Leave battle
                </button>
              ) : (
                <button type="button" className="hud-menu-item" onClick={() => setAsking("leave")}>
                  Main menu
                </button>
              )}
            </>
          )}
        </div>
      </div>
      {guideOpen && (
        <div className="hud-veil hud-modal-veil" data-occludes-readouts>
          <div className="hud-panel hud-modal" role="dialog" aria-modal="true" aria-label="Guide">
            <header className="hud-modal-head">
              <strong className="hud-menu-title">Guide</strong>
              <button
                type="button"
                className="hud-menu-item"
                onClick={() => setGuideOpen(false)}
                autoFocus
              >
                Close guide
              </button>
            </header>
            <Guide />
          </div>
        </div>
      )}
      {settingsOpen && (
        <div className="hud-veil hud-modal-veil" data-occludes-readouts>
          <div
            className="hud-panel hud-modal hud-settings-modal"
            role="dialog"
            aria-modal="true"
            aria-label="Settings"
          >
            <header className="hud-modal-head">
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
              <DiagnosticsControls>
                {onSaveReplay && <SaveReplayButton save={onSaveReplay} />}
              </DiagnosticsControls>
            </section>
          </div>
        </div>
      )}
    </>
  );
}
