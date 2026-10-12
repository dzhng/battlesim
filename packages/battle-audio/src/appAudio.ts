import type { AudioPresentation } from "./audioPresentation";
import { BattleAudio } from "./battleAudio";
import type { SoundCatalog } from "./catalog";
import type { SoundFrameOptions } from "./soundFrame";
import { soundSettings } from "./settings";
import { SoundBank } from "./soundBank";
import { WebAudioSink } from "./webAudioSink";
import { BattleMusic } from "./music/battleMusic";
import { MusicPlayer } from "./music/musicPlayer";
import type { VoiceRequest } from "./voices/chatter";

/** The menu's countryside bed, under its music (the catalog's `menu` mood). */
export const MENU_AMBIENCE = { sound: "countryside", gain: 0.08 } as const;
export const MENU_CUES = {
  hover: { sound: "recorded-clack-14", gain: 0.12 },
  click: { sound: "recorded-clack-17", gain: 0.3 },
} as const;
export const MENU_FADE_S = 1.2;

export type AudioScreen = "menu" | "loading" | "other";

/** Page lifetime, shared by menu and battles. Context activation remains tied
 * to real browser permission; navigation never closes or recreates it. */
export class AppAudio {
  private context: AudioContext | null = null;
  private bank: SoundBank | null = null;
  private menu: WebAudioSink | null = null;
  private music: MusicPlayer | null = null;
  /** The running battle's score, from its first drawn update. */
  private score: BattleMusic | null = null;
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
  private readonly onMenuInput = (event: Event) => {
    const target =
      event.target instanceof Element
        ? event.target.closest<HTMLElement>("button, a[href], input")
        : null;
    if (
      !target?.closest(".menu, .hud-menu") ||
      target.closest('[inert], [aria-disabled="true"]') ||
      target.matches(":disabled")
    )
      return;
    if (
      event.type === "pointerover" &&
      event instanceof MouseEvent &&
      event.relatedTarget instanceof Node &&
      target.contains(event.relatedTarget)
    )
      return;
    this.playMenuCue(event.type === "click" ? "click" : "hover");
  };
  private readonly onPageHide = () => this.dispose();

  constructor(
    private readonly options: { presentation: AudioPresentation; catalog: SoundCatalog },
  ) {
    this.unsubscribe = soundSettings.subscribe(() => this.applySettings());
    for (const type of ["pointerdown", "keydown"] as const)
      window.addEventListener(type, this.onGesture, { capture: true });
    for (const type of ["pointerover", "click"])
      window.addEventListener(type, this.onMenuInput, { capture: true });
    window.addEventListener("pagehide", this.onPageHide);
  }

  setScreen(screen: AudioScreen) {
    this.screen = screen;
    if (screen === "menu") {
      this.combatStarted = false;
      this.endScore(0);
    }
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
        this.bank = new SoundBank(this.context, this.options.catalog);
        this.menu = new WebAudioSink(
          this.context,
          this.options.presentation,
          this.options.catalog,
          this.bank,
        );
        this.music = new MusicPlayer(
          this.context,
          this.options.catalog.music.tracks,
          this.menu.master,
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
    // Media autoplay is admitted per element: retry inside this input too.
    this.music?.resume();
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
    this.battle?.setVolume(soundSettings.get().volume, soundSettings.get().voices);
    this.syncMenu();
  }

  private playMenuCue(kind: keyof typeof MENU_CUES) {
    if (
      this.closed ||
      soundSettings.get().muted ||
      !this.menuReady ||
      this.context?.state !== "running" ||
      !this.menu
    )
      return;
    const at = this.menu.now();
    // One transient slot bounds rapid pointer sweeps and lets activation win.
    this.menu.stop(3, at, 0);
    this.menu.start(3, {
      ...MENU_CUES[kind],
      variant: 0,
      bus: "effects",
      at,
      rate: 1,
      lowpass: 20000,
      wet: 0,
      attack: 0,
      position: null,
      pan: null,
      loop: false,
    });
  }

  createBattle(options: Omit<SoundFrameOptions, "catalog">) {
    this.battle?.dispose();
    const battle = new BattleAudio({ ...options, catalog: this.options.catalog }, this);
    this.battle = battle;
    this.combatStarted = false;
    this.start();
    return battle;
  }

  /** A running, prepared battle's drawn audio update, with the barks it
   *  heard and whether a voice line sounds. The first ends the menu's music
   *  and opens the battle's score, which the barks direct and the lines duck. */
  battleHeard(battle: BattleAudio, barks: readonly VoiceRequest[], speaking: boolean) {
    if (this.battle !== battle || !this.context) return;
    if (!this.combatStarted) {
      this.combatStarted = true;
      this.syncMenu();
      const { music } = this.options.presentation;
      const { tension, combat } = this.options.catalog.music.moods;
      // Each battle's score shuffles by its own seed, drawn by the page.
      this.score = this.music
        ? new BattleMusic(music, { tension, combat }, this.music, Math.random() * 2 ** 32)
        : null;
    }
    this.score?.update(this.context.currentTime, barks, speaking);
  }
  releaseBattle(battle: BattleAudio) {
    if (this.battle !== battle) return;
    this.battle = null;
    this.endScore(MENU_FADE_S);
  }

  /** The battle's score fades out over `fade` seconds, undipped. */
  private endScore(fade: number) {
    if (!this.score) return;
    this.score.dispose();
    this.score = null;
    this.music?.duck(0, 0);
    this.music?.stop(fade);
  }

  private prepareMenu() {
    if (!this.bank || this.menuReady || this.menuPreparing || this.error || this.screen === "other")
      return;
    this.menuPreparing = true;
    void this.bank
      .prepare([MENU_AMBIENCE.sound, ...Object.values(MENU_CUES).map(({ sound }) => sound)])
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
      this.music?.stop(this.combatStarted ? MENU_FADE_S : 0);
      this.menu.stop(1, at, this.combatStarted ? MENU_FADE_S : 0);
      return;
    }
    // A quick return during the battle fade replaces it rather than stacking beds.
    this.menu.clear();
    this.music?.stop();
    const bed = this.options.catalog.music.moods.menu[0];
    if (bed) this.music?.play(bed, { fade: 0.5 });
    this.menu.start(1, {
      sound: MENU_AMBIENCE.sound,
      variant: 0,
      bus: "ambience",
      at,
      gain: MENU_AMBIENCE.gain,
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
    const settings = soundSettings.get();
    this.menu.master.gain.setTargetAtTime(
      this.options.presentation.buses.master * settings.volume,
      this.context.currentTime,
      0.02,
    );
    // Music feeds this master, so it is master × music.
    this.music?.setLevel(this.options.presentation.buses.music * settings.music);
  }
  private applySettings() {
    if (this.closed) return;
    this.setVolume();
    this.battle?.setVolume(soundSettings.get().volume, soundSettings.get().voices);
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
    for (const type of ["pointerover", "click"])
      window.removeEventListener(type, this.onMenuInput, { capture: true });
    window.removeEventListener("pagehide", this.onPageHide);
    this.battle?.dispose();
    this.menu?.dispose();
    this.music?.dispose();
    this.bank?.dispose();
    void this.context?.close().catch(() => {});
    this.menu = null;
    this.music = null;
    this.bank = null;
    this.context = null;
    this.menuPlaying = false;
  }
}
