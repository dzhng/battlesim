import { shippedRules } from "@web/shippedRules";
import { editGameView } from "@web/battle/catalog/sets";
import { MECHANICS_API, type MechanicsSnapshot } from "@apps/mechanics-editor/src/protocol";

function freeze(value: unknown): void {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return;
  for (const child of Object.values(value)) freeze(child);
  Object.freeze(value);
}

function replace(target: object, source: object): void {
  for (const key of Object.keys(target)) Reflect.deleteProperty(target, key);
  Object.assign(target, source);
}

/** Install one accepted generation before importing any gameplay consumers.
 * Those consumers keep their imported objects for the whole page's lifetime. */
export async function startWithMechanics(
  development: boolean,
  start: () => void | Promise<void>,
  fetcher: typeof fetch = fetch,
): Promise<void> {
  if (development) {
    const response = await fetcher(MECHANICS_API, { cache: "no-store" });
    if (!response.ok)
      throw new Error(
        `Mechanics could not be loaded (${response.status}): ${await response.text()}`,
      );
    const snapshot: MechanicsSnapshot = await response.json();
    const rules = snapshot.documents.find((document) => document.path === "fixtures/rules.json");
    if (!rules) throw new Error("The mechanics snapshot has no game rules");
    replace(shippedRules, rules.value);
    editGameView((view) => replace(view, snapshot.game));
  }
  freeze(shippedRules);
  editGameView(freeze);
  await start();
}
