// P23: one fixed two-human battle, removed when rooms own entry to play.
import { preset, setOccupant } from "../../../packages/skirmish-setup/src/lobby.ts";
import { toBattleSetup } from "../../../packages/multiplayer/src/setup.ts";
import type { Seat } from "../../../packages/multiplayer/src/link.ts";
import type { BattleSetup, BattleTicket } from "../../../packages/multiplayer/src/wire.ts";

export function devBattle(start: (setup: BattleSetup) => Promise<{ mapIdentity: string }>) {
  const lobby = preset("1v1");
  const red = lobby.seats.find((seat) => seat.side === "red")!;
  const setup = toBattleSetup(
    setOccupant(lobby, red.id, { kind: "human", who: "dev-red" }),
    "skirmish",
  );
  const seats = new Map<string, Seat>();
  let ready: Promise<{ mapIdentity: string }> | undefined;
  return {
    seatOf: (browserId: string) => seats.get(browserId),
    async join(browserId: string): Promise<BattleTicket | { refused: "full" }> {
      let seat = seats.get(browserId);
      if (!seat) {
        const next = setup.seats[seats.size];
        if (!next) return { refused: "full" };
        seat = { side: next.side, player: next.player };
        seats.set(browserId, seat);
      }
      ready ??= start(setup);
      const { mapIdentity } = await ready;
      return { battle: "dev", setup, mapIdentity, ...seat };
    },
  };
}
