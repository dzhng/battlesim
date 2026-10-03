import sounds from "@fixtures/sounds.json";
import { validateSoundCatalog } from "./catalog";

/** The page's accepted audio generation, captured before consumers load. */
export const gameSounds = validateSoundCatalog(sounds);
