// The units' voices for one battle: their answers to their owner (R1–R12),
// and the sets their shouts are spoken in.
import { useEffect, type RefObject } from "react";
import { assignments as voiceAssignments } from "@fixtures/voices.json";
import { factionVoices } from "@packages/battle-audio/src/voices/voiceCatalog";
import type { Faction, UnitCatalog } from "@packages/scene-assets/src/units";
import type { SeatClient } from "@web/battle/sim/client";
import type { BattleAudio } from "@packages/battle-audio/src/battleAudio";
import { UnitResponses } from "./unitResponses";

/** Keeps `responses` answering for the current battle's units, and hands the
 *  audio each kind's voice set for the player's faction and the enemy's. */
export function useUnitVoices(
  responses: RefObject<UnitResponses | null>,
  audio: BattleAudio | null,
  client: SeatClient | null,
  units: UnitCatalog,
  faction: Faction | null,
  enemy: Faction | null,
  tickHz: number,
) {
  useEffect(() => {
    responses.current = audio && client ? new UnitResponses(audio, units, faction, tickHz) : null;
  }, [responses, audio, client, units, faction, tickHz]);
  // A shout heard in the open is a bark in its kind's set for its side's
  // faction; an unseen enemy's is drawn from the enemy faction's sets.
  // Outside a skirmish the factions are unknown, so shouts are silent.
  useEffect(() => {
    if (!audio || !faction || !enemy) return;
    const rows = units.ids.map((id) => ({ id, factions: units.type(id).roster?.factions ?? [] }));
    audio.speakers({
      own: factionVoices(voiceAssignments, rows, faction),
      enemy: factionVoices(voiceAssignments, rows, enemy),
    });
  }, [audio, units, faction, enemy]);
}
