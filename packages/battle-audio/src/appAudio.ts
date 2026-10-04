import type { AudioPresentation } from "./audioPresentation";
import { BattleAudio } from "./battleAudio";
import type { SoundCatalog } from "./catalog";
import type { SoundFrameOptions } from "./soundFrame";
import { soundSettings } from "./settings";
import { gameSounds } from "./shippedSounds";
import { SoundBank } from "./soundBank";
import { WebAudioSink } from "./webAudioSink";

/** Reversible menu mix, also used by offline listening review. */
export const MENU_BED = [
  { sound: "menu_music", gain: 1.1 },
  { sound: "countryside", gain: 0.08 },
] as const;
export const MENU_FADE_S = 1.2;

export type AudioScreen = "menu" | "loading" | "other";

/** Page lifetime, shared by menu and battles. Context activation remains tied
 * to real browser permission; navigation never closes or recreates it. */
export class AppAudio {
  private context: AudioContext | null = null;
  private bank: SoundBank | null = null;
  private menu: WebAudioSink | null = null;
  private battle: BattleAudio | null = null;
  private screen: AudioScreen = "other";
  private menuReady = false;
  private menuPreparing = false;
  private menuPlaying = false;
  private combatStarted = false;
  private error: string | null = null;
  private closed = false;
  private muted = soundSettings.get().muted;
  private readonly unsubscribe: () => void;
  private readonly onGesture = (event: Event) => {
    if (
      event instanceof KeyboardEvent &&
      (event.repeat || event.ctrlKey || event.metaKey || event.altKey)
    )
      return;
    this.start();
  };
  private readonly onPageHide = () => this.dispose();

  constructor(
    private readonly options: { presentation: AudioPresentation; catalog?: SoundCatalog },
  ) {
    this.unsubscribe = soundSettings.subscribe(() => this.applySettings());
    for (const type of ["pointerdown", "keydown"] as const)
      window.addEventListener(type, this.onGesture, { capture: true });
    window.addEventListener("pagehide", this.onPageHide);
  }

  setScreen(screen: AudioScreen) {
    this.screen = screen;
    if (screen === "menu") this.combatStarted = false;
    if (screen === "menu" || screen === "loading") this.start();
    this.syncMenu();
  }

  /** Attempt immediately, and again synchronously inside permitted input.
   * A pending autoplay resume never blocks preparation or gesture handling. */
  start() {
    if (this.closed || soundSettings.get().muted || (this.error && !this.context)) return;
    if (!this.context) {
      try {
        this.context = new AudioContext({ latencyHint: "interactive" });
        this.bank = new SoundBank(this.context, this.options.catalog ?? gameSounds);
        this.menu = new WebAudioSink(
          this.context,
          this.options.presentation,
          this.options.catalog,
          this.bank,
        );
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        this.bank?.dispose();
        void this.context?.close().catch(() => {});
        this.context = null;
        this.bank = null;
        return;
      }
    }
    this.setVolume();
    if (this.context.state !== "running") this.battle?.reset();
    // Queue resume even when state is still running: a prior suspend may be
    // pending. Permission failure remains eligible for the next gesture.
    void this.context
      .resume()
      .then(() => {
        if (this.closed || soundSettings.get().muted || (this.error && !this.context)) return;
        this.syncMenu();
      })
      .catch(() => {});
    this.prepareMenu();
    if (this.bank) this.battle?.connect(this.context, this.bank);
    this.battle?.setVolume(soundSettings.get().volume);
    this.syncMenu();
  }

  createBattle(options: Omit<SoundFrameOptions, "catalog">) {
    this.battle?.dispose();
    const battle = new BattleAudio(
      { ...options, catalog: this.options.catalog ?? gameSounds },
      this,
    );
    this.battle = battle;
    this.combatStarted = false;
    this.start();
    return battle;
  }

  /** Only a running, prepared battle's first drawn audio update ends loading music. */
  battleStarted(battle: BattleAudio) {
    if (this.battle !== battle || this.combatStarted) return;
    this.combatStarted = true;
    this.syncMenu();
  }
  releaseBattle(battle: BattleAudio) {
    if (this.battle === battle) this.battle = null;
  }

  private prepareMenu() {
    if (!this.bank || this.menuReady || this.menuPreparing || this.error || this.screen === "other")
      return;
    this.menuPreparing = true;
    void this.bank
      .prepare(MENU_BED.map(({ sound }) => sound))
      .then(() => {
        if (this.closed) return;
        this.menuReady = true;
        this.syncMenu();
      })
      .catch((error: unknown) => {
        if (this.closed) return;
        this.error = error instanceof Error ? error.message : String(error);
        console.error("Menu sound could not be prepared:", this.error);
      });
  }

  private syncMenu() {
    if (!this.menu) return;
    const wantsBed =
      !this.closed &&
      !soundSettings.get().muted &&
      this.menuReady &&
      (this.screen === "menu" || (this.screen === "loading" && !this.combatStarted));
    if (!wantsBed && (soundSettings.get().muted || this.screen === "other")) this.menu.clear();
    if (wantsBed === this.menuPlaying) return;
    this.menuPlaying = wantsBed;
    const at = this.menu.now();
    if (!wantsBed) {
      this.menu.stop(1, at, this.combatStarted ? MENU_FADE_S : 0);
      this.menu.stop(2, at, this.combatStarted ? MENU_FADE_S : 0);
      return;
    }
    // A quick return during the battle fade replaces it rather than stacking beds.
    this.menu.clear();
    for (const [i, { sound, gain }] of MENU_BED.entries())
      this.menu.start(i + 1, {
        sound,
        variant: 0,
        bus: "ambience",
        at,
        gain,
        rate: 1,
        lowpass: 20000,
        wet: 0,
        attack: 0.5,
        position: null,
        pan: null,
        loop: true,
      });
  }

  private setVolume() {
    if (!this.context || !this.menu) return;
    this.menu.master.gain.setTargetAtTime(
      this.options.presentation.buses.master * soundSettings.get().volume,
      this.context.currentTime,
      0.02,
    );
  }
  private applySettings() {
    if (this.closed) return;
    this.setVolume();
    this.battle?.setVolume(soundSettings.get().volume);
    if (this.muted !== soundSettings.get().muted) this.battle?.reset();
    this.muted = soundSettings.get().muted;
    if (this.muted) {
      this.syncMenu();
      void this.context?.suspend().catch(() => {});
    } else this.start();
  }

  stats() {
    return {
      running: !this.closed && !soundSettings.get().muted && this.context?.state === "running",
      menuReady: this.menuReady,
      menuPlaying: this.menuPlaying,
      menuVoices: this.menu?.live ?? 0,
      error: this.error,
      closed: this.closed,
    };
  }
  dispose() {
    if (this.closed) return;
    this.closed = true;
    this.unsubscribe();
    for (const type of ["pointerdown", "keydown"] as const)
      window.removeEventListener(type, this.onGesture, { capture: true });
    window.removeEventListener("pagehide", this.onPageHide);
    this.battle?.dispose();
    this.menu?.dispose();
    this.bank?.dispose();
    void this.context?.close().catch(() => {});
    this.menu = null;
    this.bank = null;
    this.context = null;
    this.menuPlaying = false;
  }
}
