import type { BattleSetup } from "../../../../packages/multiplayer/src/wire.ts";
import type { MapGenerator } from "../../../../web/src/maps/source.ts";
import { prepareBattle, type BattleData, type PreparedBattle } from "./runtime.ts";

export type CandidateAnswer = "pending" | "admitted" | "refused";

/** The first admission is final only when all lower candidates refused.
 *  null means unanswered; -1 means every candidate refused. */
export function chooseCandidate(answers: readonly CandidateAnswer[]): number | null {
  for (let i = 0; i < answers.length; i++) {
    if (answers[i] === "pending") return null;
    if (answers[i] === "admitted") return i;
  }
  return -1;
}

/** Inline tests have no threads; the decision is the same as for concurrent answers. */
export async function prepareInOrder(
  candidates: BattleSetup[],
  data: BattleData,
  generator?: MapGenerator,
) {
  const answers: CandidateAnswer[] = candidates.map(() => "pending");
  let prepared: PreparedBattle | undefined;
  for (let i = 0; chooseCandidate(answers) === null; i++) {
    try {
      prepared = await prepareBattle(candidates[i], data, generator);
      answers[i] = "admitted";
    } catch {
      answers[i] = "refused";
    }
  }
  const candidate = chooseCandidate(answers)!;
  if (!prepared) throw new Error("no battle candidate could be prepared");
  return { prepared, candidate };
}
