import { GAME_NAME } from "./gameName";

/** The game's name as its holo wordmark: the plate's title wherever the game
 *  introduces itself (the boot loading screen, then the main menu). */
export function Wordmark() {
  return <span className="wordmark">{GAME_NAME}</span>;
}
