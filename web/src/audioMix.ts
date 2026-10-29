import audio from "../../fixtures/audio.json";
import type { AudioPresentation } from "@packages/battle-audio/src/audioPresentation";

/** The page's mix, admitted with the sound catalog by Rust's SoundFrame. */
export const audioMix = audio as unknown as AudioPresentation;
