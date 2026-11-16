// @vitest-environment node
import { expect, test } from "vitest";
import {
  chooseCandidate,
  type CandidateAnswer,
} from "../../../apps/game-server/src/battleHost/candidates";

function permutations(values: number[]): number[][] {
  if (!values.length) return [[]];
  return values.flatMap((value) =>
    permutations(values.filter((other) => other !== value)).map((rest) => [value, ...rest]),
  );
}

test("every answer pattern and arrival order chooses the lowest admitted candidate at the first knowable answer", () => {
  for (let count = 0; count <= 4; count++) {
    for (let mask = 0; mask < 2 ** count; mask++) {
      const admitted = Array.from({ length: count }, (_, i) => (mask & (1 << i)) !== 0);
      const winner = admitted.indexOf(true);
      for (const order of permutations(admitted.map((_, i) => i))) {
        const answers: CandidateAnswer[] = admitted.map(() => "pending");
        const received = new Set<number>();
        const check = () => {
          const required = winner < 0 ? count : winner + 1;
          const knowable = Array.from({ length: required }, (_, i) => i).every((i) =>
            received.has(i),
          );
          expect(
            chooseCandidate(answers),
            JSON.stringify({ admitted, order, received: [...received] }),
          ).toBe(knowable ? winner : null);
        };
        check();
        for (const candidate of order) {
          received.add(candidate);
          answers[candidate] = admitted[candidate] ? "admitted" : "refused";
          check();
        }
      }
    }
  }
});
