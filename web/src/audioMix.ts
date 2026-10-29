import audio from "../../fixtures/audio.json";
import sounds from "../../fixtures/sounds.json";
import type { AudioMix } from "@packages/battle-audio/src/audioMix";

import { check_audio_mix } from "./wasm/renderer_browser/renderer_browser.js";
import { loadPresentationModule } from "./presentationModule";
await loadPresentationModule();
check_audio_mix(JSON.stringify(audio), JSON.stringify(sounds));

/** The page's mix; SoundFrame also admits its references against the library. */
export const audioMix = audio as unknown as AudioMix;
