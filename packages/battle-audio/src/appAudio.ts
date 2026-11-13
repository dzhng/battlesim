import type { AudioMix } from "./audioMix";
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
export const MENU_AMBIENCE = { sound: "countryside", gain: 0.02 } as const;
export const MENU_CUES = {
  hover: { sound: "recorded-clack-14", gain: 0.12 },
  click: { sound: "recorded-clack-17", gain: 0.3 },
} as const;
export const MENU_FADE_S = 1.2;

/** A loop outside battle (the Armory's idle engine): a sound name already
 *  resolved by the presentation, at its gain and playback rate. */
export interface ShowLoop {
  sound: string;
  gain: number;
  rate: number;
}
/** Seconds a show loop fades in and out over. */
const SHOW_FADE_S = 0.5;
/** Show loops take menu-sink voice ids from here up; the bed and the cue sit below. */
const SHOW_VOICE_BASE = 16;
const isShowVoice = (id: number | null) => id !== null && id >= SHOW_VOICE_BASE;

export type AudioScreen = "menu" | "loading" | "other";

interface AudioGraph {
  readonly context: AudioContext;
  readonly bank: SoundBank;
  readonly menu: WebAudioSink;
  readonly music: MusicPlayer;
  dispose(): void;
}

type MenuPreparation =
  | { status: "unprepared" }
  | { status: "preparing" }
  | { status: "ready" }
  | { status: "failed"; error: string };

/** Page lifetime, shared by menu and battles. Context activation remains tied
 * to real browser permission; navigation never closes or recreates it. */
export class AppAudio {
  private graph: AudioGraph | null = null;
  /** The running battle's score, from its first drawn update. */
  private score: BattleMusic | null = null;
  private battle: BattleAudio | null = null;
  /** The battle's seed, which its score shuffles by. */
  private scoreSeed = 0;
  private screen: AudioScreen = "other";
  private menuPreparation: MenuPreparation = { status: "unprepared" };
  private menuPlaying = false;
  /** The show loops asked for; forgotten when the menu screen ends. */
  private showWanted: readonly ShowLoop[] = [];
  /** Each sounding show loop's menu-sink voice, by sound. */
  private readonly showVoices = new Map<string, number>();
  private nextShowVoice = SHOW_VOICE_BASE;
  private combatStarted = false;
  private readonly embeddedSources = new Set<AudioBufferSourceNode>();
  private contextError: string | null = null;
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
    if (event.type === "click") {
      this.playMenuCue("click");
      return;
    }
    // A quiet region (a list the pointer sweeps) keeps its clicks only.
    if (target.closest('[data-cue-hover="off"]')) return;
    if (
      event instanceof MouseEvent &&
      event.relatedTarget instanceof Node &&
      target.contains(event.relatedTarget)
    )
      return;
    this.playMenuCue("hover");
  };
  private readonly onPageHide = () => this.dispose();

  constructor(private readonly options: { presentation: AudioMix; catalog: SoundCatalog }) {
    this.unsubscribe = soundSettings.subscribe(() => this.applySettings());
    for (const type of ["pointerdown", "keydown"] as const)
      window.addEventListener(type, this.onGesture, { capture: true });
    for (const type of ["pointerover", "click"])
      window.addEventListener(type, this.onMenuInput, { capture: true });
    window.addEventListener("pagehide", this.onPageHide);
  }

  setScreen(screen: AudioScreen) {
    this.screen = screen;
    if (screen !== "menu") this.showWanted = [];
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
    if (this.closed || soundSettings.get().muted || (this.contextError && !this.graph)) return;
    if (!this.graph) {
      let context: AudioContext | null = null;
      let bank: SoundBank | null = null;
      let menu: WebAudioSink | null = null;
      let music: MusicPlayer | null = null;
      try {
        context = new AudioContext({ latencyHint: "interactive" });
        bank = new SoundBank(context, this.options.catalog);
        menu = new WebAudioSink(context, this.options.presentation, this.options.catalog, bank);
        music = new MusicPlayer(context, this.options.catalog.music.tracks, menu.master);
        this.graph = {
          context,
          bank,
          menu,
          music,
          dispose: () => {
            menu?.dispose();
            music?.dispose();
            bank?.dispose();
            void context?.close().catch(() => {});
          },
        };
      } catch (error) {
        this.contextError = error instanceof Error ? error.message : String(error);
        menu?.dispose();
        bank?.dispose();
        void context?.close().catch(() => {});
        return;
      }
    }
    const graph = this.graph;
    if (!graph) return;
    this.setVolume();
    // Media autoplay is admitted per element: retry inside this input too.
    graph.music.resume();
    if (graph.context.state !== "running") this.battle?.reset();
    // Queue resume even when state is still running: a prior suspend may be
    // pending. Permission failure remains eligible for the next gesture.
    void graph.context
      .resume()
      .then(() => {
        if (this.closed || soundSettings.get().muted || (this.contextError && !this.graph)) return;
        this.syncMenu();
      })
      .catch(() => {});
    this.prepareMenu();
    this.battle?.connect(graph.context, graph.bank);
    this.battle?.setVolume(soundSettings.get().volume, soundSettings.get().voices);
    this.syncMenu();
  }

  private playMenuCue(kind: keyof typeof MENU_CUES) {
    const graph = this.graph;
    if (
      this.closed ||
      soundSettings.get().muted ||
      this.menuPreparation.status !== "ready" ||
      !graph ||
      graph.context.state !== "running"
    )
      return;
    const menu = graph.menu;
    const at = menu.now();
    // One transient slot bounds rapid pointer sweeps and lets activation win.
    menu.stop(3, at, 0);
    menu.start(3, {
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

  /** The non-battle loops now playing, beside the menu bed; [] fades them
   *  all out. A sound that stays keeps its voice and glides to its new gain
   *  and rate. Leaving the menu screen ends them; mute holds them. */
  showLoops(loops: readonly ShowLoop[]) {
    this.showWanted = loops;
    this.syncShow();
  }

  /** A battle's sound; its score shuffles by `seed`, the battle's own, so a
   *  replayed battle (a capture's scene) plays the same score. */
  createBattle(options: Omit<SoundFrameOptions, "catalog">, seed: number) {
    this.battle?.dispose();
    this.scoreSeed = seed;
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
    const graph = this.graph;
    if (this.battle !== battle || !graph) return;
    if (!this.combatStarted) {
      this.combatStarted = true;
      this.syncMenu();
      const { music } = this.options.presentation;
      const { tension, combat } = this.options.catalog.music.moods;
      this.score = new BattleMusic(music, { tension, combat }, graph.music, this.scoreSeed);
    }
    this.score?.update(graph.context.currentTime, barks, speaking);
  }

  /** Play one validated, embedded scenario clip through the existing effects
   * bus. The scenario owns the bytes; AppAudio owns decoding and disposal. */
  async playEmbeddedAudio(bytesBase64: string, mediaType: string): Promise<void> {
    if (!mediaType.startsWith("audio/")) throw new Error("embedded scenario asset is not audio");
    this.start();
    const context = this.graph?.context;
    const sink = this.graph?.menu;
    if (!context || !sink) throw new Error("audio context is unavailable");
    const binary = atob(bytesBase64);
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    const decoded = await context.decodeAudioData(bytes.buffer.slice(0));
    if (this.closed || this.graph?.menu !== sink) return;
    const source = new AudioBufferSourceNode(context, { buffer: decoded });
    source.connect(sink.buses.effects);
    this.embeddedSources.add(source);
    source.onended = () => {
      this.embeddedSources.delete(source);
      source.disconnect();
    };
    source.start();
  }

  /** Only a running, prepared battle's first drawn audio update ends loading music. */
  battleStarted(battle: BattleAudio) {
    if (this.battle !== battle || this.combatStarted) return;
    this.combatStarted = true;
    this.syncMenu();
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
    this.graph?.music.duck(0, 0);
    this.graph?.music.stop(fade);
  }

  private prepareMenu() {
    const graph = this.graph;
    if (!graph || this.menuPreparation.status !== "unprepared" || this.screen === "other") return;
    this.menuPreparation = { status: "preparing" };
    void graph.bank
      .prepare([MENU_AMBIENCE.sound, ...Object.values(MENU_CUES).map(({ sound }) => sound)])
      .then(() => {
        if (this.closed) return;
        this.menuPreparation = { status: "ready" };
        this.syncMenu();
      })
      .catch((error: unknown) => {
        if (this.closed) return;
        const message = error instanceof Error ? error.message : String(error);
        this.menuPreparation = { status: "failed", error: message };
        console.error("Menu sound could not be prepared:", message);
      });
  }

  private syncMenu() {
    this.syncBed();
    this.syncShow();
  }

  private syncBed() {
    const graph = this.graph;
    if (!graph) return;
    const wantsBed =
      !this.closed &&
      !soundSettings.get().muted &&
      this.menuPreparation.status === "ready" &&
      (this.screen === "menu" || (this.screen === "loading" && !this.combatStarted));
    if (!wantsBed && (soundSettings.get().muted || this.screen === "other")) {
      graph.menu.clear();
      this.showVoices.clear();
    }
    if (wantsBed === this.menuPlaying) return;
    this.menuPlaying = wantsBed;
    const at = graph.menu.now();
    if (!wantsBed) {
      graph.music.stop(this.combatStarted ? MENU_FADE_S : 0);
      graph.menu.stop(1, at, this.combatStarted ? MENU_FADE_S : 0);
      return;
    }
    // A quick return during the battle fade replaces it rather than stacking beds.
    graph.menu.clear((id) => !isShowVoice(id));
    graph.music.stop();
    const bed = this.options.catalog.music.moods.menu[0];
    if (bed) graph.music.play(bed, { fade: 0.5 });
    graph.menu.start(1, {
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

  /** Start, retune or fade show voices toward the loops asked for, once
   *  their sounds are prepared. */
  private syncShow() {
    const graph = this.graph;
    if (!graph) return;
    const wanted = this.closed || soundSettings.get().muted ? [] : this.showWanted;
    const missing = wanted.map(({ sound }) => sound).filter((s) => !graph.bank.prepared(s));
    if (missing.length > 0)
      void graph.bank.prepare(missing).then(
        () => this.syncShow(),
        (error: unknown) => {
          if (this.closed) return;
          const message = error instanceof Error ? error.message : String(error);
          console.error("Show sound could not be prepared:", message);
        },
      );
    const ready = new Map(
      wanted.filter(({ sound }) => graph.bank.prepared(sound)).map((l) => [l.sound, l]),
    );
    const at = graph.menu.now();
    for (const [sound, id] of this.showVoices) {
      if (ready.has(sound)) continue;
      graph.menu.stop(id, at, SHOW_FADE_S);
      this.showVoices.delete(sound);
    }
    for (const [sound, { gain, rate }] of ready) {
      const params = { gain, rate, lowpass: 20000, wet: 0, position: null };
      const id = this.showVoices.get(sound);
      if (id !== undefined) {
        graph.menu.set(id, params, at);
        continue;
      }
      const voice = this.nextShowVoice++;
      this.showVoices.set(sound, voice);
      graph.menu.start(voice, {
        ...params,
        sound,
        variant: 0,
        bus: "units",
        at,
        attack: SHOW_FADE_S,
        pan: null,
        loop: true,
      });
    }
  }

  private setVolume() {
    const graph = this.graph;
    if (!graph) return;
    const settings = soundSettings.get();
    graph.menu.master.gain.setTargetAtTime(
      this.options.presentation.buses.master * settings.volume,
      graph.context.currentTime,
      0.02,
    );
    // Music feeds this master, so it is master × music.
    graph.music.setLevel(this.options.presentation.buses.music * settings.music);
  }
  private applySettings() {
    if (this.closed) return;
    this.setVolume();
    this.battle?.setVolume(soundSettings.get().volume, soundSettings.get().voices);
    if (this.muted !== soundSettings.get().muted) this.battle?.reset();
    this.muted = soundSettings.get().muted;
    if (this.muted) {
      this.syncMenu();
      void this.graph?.context.suspend().catch(() => {});
    } else this.start();
  }

  stats() {
    return {
      running:
        !this.closed && !soundSettings.get().muted && this.graph?.context.state === "running",
      menuReady: this.menuPreparation.status === "ready",
      menuPlaying: this.menuPlaying,
      menuVoices: this.graph?.menu.live ?? 0,
      error:
        this.contextError ??
        (this.menuPreparation.status === "failed" ? this.menuPreparation.error : null),
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
    for (const source of this.embeddedSources) {
      try {
        source.stop();
      } catch {
        /* Already stopped. */
      }
      source.disconnect();
    }
    this.embeddedSources.clear();
    this.graph?.dispose();
    this.graph = null;
    this.menuPlaying = false;
  }
}
