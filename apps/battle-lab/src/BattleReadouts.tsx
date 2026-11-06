import type { ComponentProps } from "react";
import type { ReadoutLayer } from "@web/battle/present/readouts";
import type { BattleSession } from "./useBattleSession";

/** The battle view's unit info cards, from the side's own view: its units
 *  (an ally's naming its owner), the enemies it has identified, and its
 *  presented contacts, a remembered one fading as it retires. The battle
 *  view and the graphics test's capture draw cards only as
 *  `<ReadoutLayer {...battleReadouts(session)} />`, so a filmed card is the
 *  one a player sees. Place them each frame with `session.placePanels`.
 *
 *  This picks the layer's props rather than being a component that takes the
 *  session: the session is a new object every render, and React's development
 *  build would log it, walked deep, into every commit's `performance.measure`
 *  detail (`feed.ts`), which the scene runner fails. */
export function battleReadouts(session: BattleSession): ComponentProps<typeof ReadoutLayer> {
  const { observation } = session.sim;
  return {
    own: observation?.own ?? [],
    identified: observation?.identified,
    contacts: session.contacts,
    tick: observation?.tick,
    rules: session.rules,
    selected: session.control.selected,
    handle: session.readouts,
    ghosts: session.ghostReadouts,
    allies: session.allies,
  };
}
