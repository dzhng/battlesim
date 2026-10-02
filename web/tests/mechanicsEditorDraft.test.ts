// @vitest-environment node
import { expect, it } from "vitest";
import { editDraft, draftEntry, restoreDraft } from "../../apps/mechanics-editor/src/draft";
import type { MechanicsDraft } from "../../apps/mechanics-editor/src/protocol";

it("preserves landing spread when range changes, through subsequent spread and raw edits", () => {
  const target = { section: "weapons" as const, id: "test" };
  const original = { range_m: 300, scatter_mrad: 15 };
  let draft: MechanicsDraft = { revision: "one", changes: [] };
  draft = editDraft(draft, target, ["range_m"], 150, original);
  expect(draftEntry(original, draft, target)).toEqual({ range_m: 150, scatter_mrad: 30 });
  draft = editDraft(draft, target, ["scatter_mrad"], 20, draftEntry(original, draft, target));
  draft = editDraft(draft, target, ["range_m"], 300, draftEntry(original, draft, target));
  expect(draftEntry(original, draft, target)).toEqual({ range_m: 300, scatter_mrad: 10 });
  expect(draft.changes.filter((c) => c.path[0] === "range_m")).toEqual([
    { ...target, path: ["range_m"], value: 300 },
  ]);
});

it("isolates matching soldier-kind edits to a selected unit and replaces earlier drafts by path", () => {
  const original = { hp: 100, mounts: [{ name: "rifles", weapons: ["rifle"] }] };
  const rifleTarget = { section: "soldiers" as const, id: "rifleman", unit: "rifle" };
  const atTarget = { ...rifleTarget, unit: "at" };
  let draft = editDraft({ revision: "one", changes: [] }, rifleTarget, ["hp"], 80, original);
  draft = editDraft(draft, rifleTarget, ["hp"], 90, original);
  expect(draftEntry(original, draft, rifleTarget).hp).toBe(90);
  expect(draftEntry(original, draft, atTarget).hp).toBe(100);
  draft = restoreDraft(draft, rifleTarget, ["hp"]);
  expect(draft.changes).toEqual([{ ...rifleTarget, path: ["hp"], restore: true }]);
});

it("reads source provenance and restores only the named mount override", async () => {
  const { fieldOrigin } = await import("../../apps/mechanics-editor/src/draft");
  const snapshot = {
    revision: "one",
    catalog: {},
    documents: [
      {
        path: "family.json",
        value: {
          units: {
            base: {
              mounts: [{ name: "cannon", weapons: ["ap"], turret: true }],
              sensors: { ground_m: 600 },
            },
            variant: { extends: "base", mounts: [{ name: "cannon", weapons: ["he"] }] },
          },
        },
      },
    ],
  };
  const target = { section: "units" as const, id: "variant" };
  expect(fieldOrigin(snapshot, target, ["sensors", "ground_m"])).toMatchObject({
    id: "base",
    path: "family.json",
    inherited: true,
    local: false,
  });
  expect(fieldOrigin(snapshot, target, ["mounts", "cannon", "weapons"])).toMatchObject({
    id: "variant",
    canRestore: true,
    parentValue: ["ap"],
  });
  const draft = restoreDraft({ revision: "one", changes: [] }, target, [
    "mounts",
    "cannon",
    "weapons",
  ]);
  expect(
    draftEntry(
      { mounts: [{ name: "cannon", weapons: ["he"], turret: true }] },
      draft,
      target,
      snapshot,
    ),
  ).toEqual({ mounts: [{ name: "cannon", weapons: ["ap"], turret: true }] });
});
