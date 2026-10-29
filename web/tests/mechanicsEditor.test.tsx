import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Routes, Route } from "react-router";
import MechanicsEditor from "../../apps/mechanics-editor/src/MechanicsEditor";
import gameCatalog from "../../fixtures/catalog.json";
import infantry from "../../fixtures/units/test/infantry.json";
import soldiers from "../../fixtures/units/test/soldiers.json";
import tanks from "../../fixtures/units/test/tanks.json";
import game from "../../fixtures/game.json";
import type { JsonObject, MechanicsSnapshot } from "../../apps/mechanics-editor/src/protocol";
import { UNITS, WEAPONS } from "./catalog";

/** Every editable unit, the test units included, as the editor resolves them. */
const catalog = { ...UNITS.view, weapons: WEAPONS };

const snapshot: MechanicsSnapshot = {
  revision: "test-one",
  catalog: catalog as unknown as JsonObject,
  game: gameCatalog as unknown as JsonObject,
  documents: [
    { path: "fixtures/units/test/infantry.json", value: infantry },
    { path: "fixtures/units/test/soldiers.json", value: soldiers },
    { path: "fixtures/units/test/tanks.json", value: tanks as unknown as JsonObject },
    { path: "fixtures/game.json", value: game },
  ],
};
const structuralSnapshot = (): MechanicsSnapshot => {
  const fixture = structuredClone(snapshot);
  delete fixture.catalog.documents;
  const units = fixture.catalog.units as JsonObject[];
  units.find((unit) => unit.id === "test_rifle")!.body = {
    squad: { slots: ["test_rifleman", "test_grenadier"] },
  };
  fixture.catalog.soldiers = {
    test_rifleman: { hp: 100, mounts: [{ id: "rifles", weapons: ["rifle"] }] },
    test_grenadier: { hp: 100, mounts: [{ id: "launcher", weapons: ["grenade"] }] },
  };
  const weapons = fixture.catalog.weapons as Record<string, JsonObject>;
  weapons.rifle.magazine = { rounds: 30, shot_interval_s: 0.1, burst: null };
  return fixture;
};
const respond = (value: unknown) =>
  new Response(JSON.stringify(value), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

it("shows exact units by default and preserves an inline edit while filtering", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(snapshot)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  await screen.findByRole("heading", { name: "Mechanics" });
  expect(await screen.findByRole("button", { name: "Expand Rifle Section" })).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Rifle Section deployment cost"), {
    target: { value: "125" },
  });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "test_tank" } });
  expect(screen.queryByRole("button", { name: "Expand Rifle Section" })).toBeNull();
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
  expect((screen.getByLabelText("Rifle Section deployment cost") as HTMLInputElement).value).toBe(
    "125",
  );
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    false,
  );
});

it("edits each layer range and spread independently", async () => {
  const fixture = structuredClone(snapshot);
  const weapons = fixture.catalog.weapons as Record<string, JsonObject>;
  weapons.grenade.targets = {
    ground: { classes: ["infantry", "vehicle"], range_m: 300, scatter_mrad: 15 },
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(fixture)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  fireEvent.click(screen.getByText("Grenade", { selector: "summary" }));
  const range = screen.getByLabelText("grenade Range on ground") as HTMLInputElement;
  const spread = screen.getByLabelText("grenade Spread on ground") as HTMLInputElement;
  expect(range.value).toBe("300");
  expect(spread.value).toBe("15");
  fireEvent.change(range, { target: { value: "150" } });
  expect(spread.value).toBe("15");
  fireEvent.change(spread, { target: { value: "20" } });
  expect(range.value).toBe("150");
  fireEvent.click(range.closest(".me-field")!.querySelector("button")!);
  expect(range.value).toBe("300");
  expect(spread.value).toBe("20");
  fireEvent.click(spread.closest(".me-field")!.querySelector("button")!);
  expect(range.value).toBe("300");
  expect(spread.value).toBe("15");
});

it("edits a high-air-only weapon's layer range and spread", async () => {
  const fixture = structuredClone(snapshot);
  const weapons = fixture.catalog.weapons as Record<string, JsonObject>;
  weapons.grenade.targets = {
    high_air: { classes: ["air_fixed_wing"], range_m: 800, scatter_mrad: 6 },
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(fixture)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  fireEvent.click(screen.getByText("Grenade", { selector: "summary" }));
  const range = screen.getByLabelText("grenade Range in high air") as HTMLInputElement;
  const spread = screen.getByLabelText("grenade Spread in high air") as HTMLInputElement;
  expect(range.value).toBe("800");
  expect(spread.value).toBe("6");
  fireEvent.change(range, { target: { value: "700" } });
  fireEvent.change(spread, { target: { value: "8" } });
  expect(range.value).toBe("700");
  expect(spread.value).toBe("8");
});

it.each([150, 0])("re-enables a ground layer with its admitted range %i", async (reach) => {
  const fixture = structuredClone(snapshot);
  const weapons = fixture.catalog.weapons as Record<string, JsonObject>;
  weapons.grenade.targets = {
    ground: { classes: ["infantry", "vehicle"], range_m: reach, scatter_mrad: 30 },
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(fixture)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  fireEvent.click(screen.getByText("Grenade", { selector: "summary" }));
  const enabled = screen.getByLabelText("grenade Engages ground");
  fireEvent.change(enabled, { target: { value: "" } });
  expect(screen.queryByLabelText("grenade Range on ground")).toBeNull();
  fireEvent.change(enabled, { target: { value: "enabled" } });
  expect((screen.getByLabelText("grenade Range on ground") as HTMLInputElement).value).toBe(
    String(reach),
  );
  expect((screen.getByLabelText("grenade Spread on ground") as HTMLInputElement).value).toBe("30");
});

it("undoes a child edit after re-enabling its layer without losing sibling edits", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(snapshot)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  fireEvent.click(screen.getByText("Grenade", { selector: "summary" }));
  const enabled = screen.getByLabelText("grenade Engages ground");
  fireEvent.change(enabled, { target: { value: "" } });
  fireEvent.change(enabled, { target: { value: "enabled" } });
  const range = screen.getByLabelText("grenade Range on ground") as HTMLInputElement;
  const before = range.value;
  fireEvent.change(range, { target: { value: "135" } });
  fireEvent.change(screen.getByLabelText("grenade Spread on ground"), { target: { value: "32" } });
  fireEvent.click(within(range.closest(".me-field")!).getByRole("button", { name: "Undo edit" }));
  expect(range.value).toBe(before);
  expect((screen.getByLabelText("grenade Spread on ground") as HTMLInputElement).value).toBe("32");
});

it("keeps an inherited spread editable after restoring it under a re-enabled layer", async () => {
  const fixture = structuredClone(snapshot);
  const weapons = fixture.catalog.weapons as Record<string, JsonObject>;
  const ground = (weapons.grenade.targets as JsonObject).ground as JsonObject;
  weapons.parent = { ...weapons.grenade, targets: { ground: { ...ground, scatter_mrad: 20 } } };
  fixture.documents = [
    {
      path: "fixtures/game.json",
      value: {
        weapons: {
          parent: weapons.parent,
          grenade: {
            extends: "parent",
            targets: { ground: { scatter_mrad: ground.scatter_mrad } },
          },
        },
      },
    },
  ];
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(fixture)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  fireEvent.click(screen.getByText("Grenade", { selector: "summary" }));
  const enabled = screen.getByLabelText("grenade Engages ground");
  fireEvent.change(enabled, { target: { value: "" } });
  fireEvent.change(enabled, { target: { value: "enabled" } });
  const spread = screen.getByLabelText("grenade Spread on ground");
  fireEvent.click(
    within(spread.closest(".me-field")!).getByRole("button", { name: "Restore inherited value" }),
  );
  const restored = screen.getByLabelText("grenade Spread on ground") as HTMLInputElement;
  expect(restored.value).toBe("20");
  fireEvent.change(restored, { target: { value: "24" } });
  expect(restored.value).toBe("24");
});

it("retains invalid text through filtering and refuses preview until it is corrected", async () => {
  const fetcher = vi.fn(async () => respond(snapshot));
  vi.stubGlobal("fetch", fetcher);
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  const cost = await screen.findByLabelText("Rifle Section deployment cost");
  fireEvent.change(cost, { target: { value: "120" } });
  fireEvent.change(cost, { target: { value: "12e" } });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "test_tank" } });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
  expect((screen.getByLabelText("Rifle Section deployment cost") as HTMLInputElement).value).toBe(
    "12e",
  );
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    true,
  );
  expect(fetcher.mock.calls.length).toBe(1);
  fireEvent.change(screen.getByLabelText("Rifle Section deployment cost"), {
    target: { value: "130" },
  });
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    false,
  );
});

it("requires a preview of the current draft before saving and resets only after accepted save", async () => {
  let published = false;
  const fetcher = vi.fn(async (url: string, init?: RequestInit) => {
    if (url.endsWith("/preview")) {
      const body = JSON.parse(String(init?.body));
      return respond({
        revision: snapshot.revision,
        catalog,
        affectedUnits: ["test_rifle"],
        warnings: [],
        files: [
          { path: "family.json", before: "{}\n", after: JSON.stringify(body.changes, null, 2) },
        ],
      });
    }
    if (url.endsWith("/save")) {
      published = true;
      return respond({ ...snapshot, revision: "saved" });
    }
    return respond(snapshot);
  });
  vi.stubGlobal("fetch", fetcher);
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.change(await screen.findByLabelText("Rifle Section deployment cost"), {
    target: { value: "125" },
  });
  const save = screen.getByRole("button", { name: "Save authored JSON" });
  expect(save.hasAttribute("disabled")).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("heading", { name: "Preview authored changes" });
  expect(save.hasAttribute("disabled")).toBe(false);
  fireEvent.change(screen.getByLabelText("Rifle Section deployment cost"), {
    target: { value: "130" },
  });
  expect(save.hasAttribute("disabled")).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await waitFor(() => expect(save.hasAttribute("disabled")).toBe(false));
  fireEvent.click(save);
  await screen.findByRole("status");
  expect(published).toBe(true);
  expect((screen.getByLabelText("Rifle Section deployment cost") as HTMLInputElement).value).toBe(
    "100",
  );
  const body = JSON.parse(
    String(fetcher.mock.calls.find(([url]) => url.endsWith("/save"))?.[1]?.body),
  );
  expect(body).toEqual({
    revision: "test-one",
    changes: [{ section: "units", id: "test_rifle", path: ["cost"], value: 130 }],
  });
});

it("keeps a conflicted draft until explicit discard and reload", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) =>
      url.endsWith("/preview")
        ? new Response(JSON.stringify({ error: "Revision changed on disk." }), { status: 409 })
        : respond(snapshot),
    ),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.change(await screen.findByLabelText("Rifle Section deployment cost"), {
    target: { value: "125" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByText("Sources changed outside this editor");
  expect((screen.getByLabelText("Rifle Section deployment cost") as HTMLInputElement).value).toBe(
    "125",
  );
  fireEvent.click(screen.getByRole("button", { name: "Reload sources" }));
  expect((screen.getByLabelText("Rifle Section deployment cost") as HTMLInputElement).value).toBe(
    "125",
  );
  fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
  expect(screen.queryByRole("button", { name: "Discard draft and reload" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Reload sources" }));
  fireEvent.click(screen.getByRole("button", { name: "Discard draft and reload" }));
  await waitFor(() =>
    expect((screen.getByLabelText("Rifle Section deployment cost") as HTMLInputElement).value).toBe(
      "100",
    ),
  );
});

it("discarding an optional parent clears its unfinished child text without clearing other edits", async () => {
  const fetcher = vi.fn(async () => respond(structuralSnapshot()));
  vi.stubGlobal("fetch", fetcher);
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  fireEvent.click(screen.getByText("Rifle", { selector: "summary" }));
  fireEvent.change(screen.getByLabelText("rifle Cyclic firing rate"), { target: { value: "-" } });
  fireEvent.change(screen.getByLabelText("Rifle Section deployment cost"), {
    target: { value: "12e" },
  });
  fireEvent.change(screen.getByLabelText("rifle Magazine / belt"), { target: { value: "" } });
  expect(screen.queryByLabelText("rifle Cyclic firing rate")).toBeNull();
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    true,
  );
  expect((screen.getByLabelText("Rifle Section deployment cost") as HTMLInputElement).value).toBe(
    "12e",
  );
  fireEvent.change(screen.getByLabelText("Rifle Section deployment cost"), {
    target: { value: "125" },
  });
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    false,
  );
});

it("removing the last slot of a soldier kind discards only that unit's removed soldier edits", async () => {
  const accepted = structuredClone(catalog) as unknown as JsonObject;
  const document = (accepted.documents as JsonObject[])[0];
  (document.soldiers as JsonObject).test_rifle__test_rifleman = {
    ...((document.soldiers as JsonObject).test_rifleman as JsonObject),
    hp: 110,
  };
  const fetcher = vi.fn(async (url: string, init?: RequestInit) =>
    url.endsWith("/preview")
      ? respond({
          revision: snapshot.revision,
          catalog: accepted,
          soldierIds: { '["soldiers","test_rifleman","test_rifle"]': "test_rifle__test_rifleman" },
          affectedUnits: ["test_rifle"],
          warnings: [],
          files: [{ path: "family.json", before: "{}", after: String(init?.body) }],
        })
      : respond(structuralSnapshot()),
  );
  vi.stubGlobal("fetch", fetcher);
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  fireEvent.change(screen.getByLabelText("test_grenadier Soldier health"), {
    target: { value: "125" },
  });
  fireEvent.change(screen.getByLabelText("test_grenadier Soldier health"), {
    target: { value: "-" },
  });
  fireEvent.change(screen.getByLabelText("test_rifleman Soldier health"), {
    target: { value: "110" },
  });
  fireEvent.change(screen.getByLabelText("test_rifle Soldier slots"), {
    target: { value: "test_rifleman, test_rifleman" },
  });
  expect(screen.queryByLabelText("test_grenadier Soldier health")).toBeNull();
  expect((screen.getByLabelText("test_rifleman Soldier health") as HTMLInputElement).value).toBe(
    "110",
  );
  const preview = screen.getByRole("button", { name: "Preview changes" });
  expect(preview.hasAttribute("disabled")).toBe(false);
  fireEvent.click(preview);
  await screen.findByRole("heading", { name: "Preview authored changes" });
  const request = fetcher.mock.calls.find(([url]) => url.endsWith("/preview"));
  expect(JSON.parse(String(request?.[1]?.body)).changes).toEqual([
    { section: "soldiers", id: "test_rifleman", unit: "test_rifle", path: ["hp"], value: 110 },
    {
      section: "units",
      id: "test_rifle",
      path: ["body", "squad", "slots"],
      value: ["test_rifleman", "test_rifleman"],
    },
  ]);
});

it("undoing a mount-list replacement clears unfinished edits on its discarded mounts", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(structuralSnapshot())),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Tank" }));
  const list = screen.getByLabelText("test_tank Weapon mounts") as HTMLTextAreaElement;
  const initial = JSON.parse(list.value);
  fireEvent.change(list, {
    target: {
      value: JSON.stringify([
        ...initial,
        { id: "temporary", name: "Temporary", weapons: ["rifle"], pivot_m: [0, 0, 0] },
      ]),
    },
  });
  const pivot = screen.getAllByLabelText("test_tank Pivot position").at(-1)!;
  fireEvent.change(pivot, { target: { value: "-" } });
  fireEvent.change(screen.getByLabelText("Tank deployment cost"), { target: { value: "125" } });
  fireEvent.click(list.closest(".me-field")!.querySelector("button")!);
  expect(screen.queryByRole("heading", { name: "Mount · temporary" })).toBeNull();
  expect(JSON.parse(list.value)).toEqual(initial);
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    false,
  );
});

it("restoring an inherited optional object clears unfinished child text", async () => {
  const fixture = structuralSnapshot();
  fixture.documents.unshift({
    path: "magazine.json",
    value: {
      weapons: {
        base: { magazine: null },
        rifle: { extends: "base", magazine: { rounds: 30, shot_interval_s: 0.1 } },
      },
    },
  });
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(fixture)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  fireEvent.change(screen.getByLabelText("rifle Cyclic firing rate"), { target: { value: "-" } });
  const magazine = screen.getByLabelText("rifle Magazine / belt");
  fireEvent.click(magazine.closest(".me-field")!.querySelector("button")!);
  expect(screen.queryByLabelText("rifle Cyclic firing rate")).toBeNull();
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    false,
  );
});

it("returns to the last connected gameplay edit after native rejection without guessing the failed field", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) =>
      url.endsWith("/preview")
        ? new Response(JSON.stringify({ error: "ScatterTooTight: rejected flight settings" }), {
            status: 400,
          })
        : respond(snapshot),
    ),
  );
  const scrollIntoView = HTMLElement.prototype.scrollIntoView;
  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
    configurable: true,
    value: vi.fn(),
  });
  try {
    render(<MechanicsEditor />, { wrapper: MemoryRouter });
    fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
    const spread = screen.getByLabelText("grenade Spread on ground");
    fireEvent.change(spread, { target: { value: "1" } });
    fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
    await screen.findByText("ScatterTooTight: rejected flight settings");
    const back = screen.getByRole("button", { name: "Back to last edit" });
    back.focus();
    fireEvent.click(back);
    expect(document.activeElement).toBe(spread);
    expect(spread.closest("details")!.open).toBe(true);
    expect((spread as HTMLInputElement).value).toBe("1");
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "test_tank" } });
    expect(screen.queryByRole("button", { name: "Back to last edit" })).toBeNull();
  } finally {
    Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
      configurable: true,
      value: scrollIntoView,
    });
  }
});

it("restores a saved soldier override to the native default omitted by its parent", async () => {
  const fixture = structuralSnapshot();
  const units = fixture.catalog.units as JsonObject[];
  units.find((unit) => unit.id === "test_rifle")!.body = {
    squad: { slots: ["test_rifle__test_rifleman"] },
  };
  fixture.catalog.soldiers = {
    test_rifleman: {
      hp: 100,
      mounts: [{ id: "rifles", weapons: ["rifle"], squad: false, special: false }],
    },
    test_rifle__test_rifleman: {
      hp: 100,
      mounts: [{ id: "rifles", weapons: ["rifle"], squad: false, special: true }],
    },
  };
  fixture.documents = [
    {
      path: "soldiers.json",
      value: {
        soldiers: {
          test_rifleman: {
            hp: 100,
            mounts: [{ id: "rifles", weapons: ["rifle"], squad: false }],
          },
          test_rifle__test_rifleman: {
            extends: "test_rifleman",
            mounts: [{ id: "rifles", special: true }],
          },
        },
      },
    },
  ];
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(fixture)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  const special = screen.getByLabelText("test_rifle__test_rifleman Transferable special weapon");
  fireEvent.click(
    within(special.closest(".me-field") as HTMLElement).getByRole("button", {
      name: "Restore default value",
    }),
  );
  expect((special as HTMLSelectElement).value).toBe("false");
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    false,
  );
});

it("summarizes the admitted soldier clone instead of the unfinished projection", async () => {
  const fixture = structuralSnapshot();
  fixture.catalog.soldiers = {
    test_rifleman: {
      hp: 100,
      mounts: [{ id: "rifles", weapons: ["rifle"], squad: true, special: false }],
    },
  };
  (fixture.catalog.units as JsonObject[]).find((unit) => unit.id === "test_rifle")!.body = {
    squad: { slots: ["test_rifleman"] },
  };
  const accepted = structuredClone(fixture.catalog);
  (accepted.soldiers as JsonObject).test_rifle__test_rifleman = {
    hp: 100,
    mounts: [{ id: "rifles", weapons: ["rifle"], squad: false, special: false }],
  };
  (accepted.units as JsonObject[]).find((unit) => unit.id === "test_rifle")!.body = {
    squad: { slots: ["test_rifle__test_rifleman"] },
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) =>
      url.endsWith("/preview")
        ? respond({
            revision: fixture.revision,
            catalog: accepted,
            soldierIds: {
              '["soldiers","test_rifleman","test_rifle"]': "test_rifle__test_rifleman",
            },
            affectedUnits: ["test_rifle"],
            warnings: [],
            files: [],
          })
        : respond(fixture),
    ),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  fireEvent.change(screen.getByLabelText("test_rifleman Weapon mounts"), {
    target: { value: JSON.stringify([{ id: "rifles", weapons: ["rifle"] }]) },
  });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  const summary = within(await screen.findByRole("region", { name: "Save preview" })).getByRole(
    "listitem",
  );
  expect(summary.textContent).toContain('"squad": false');
  expect(summary.textContent).toContain('"special": false');
});

it("summarizes a whole-mount restore after the accepted catalog removes its soldier clone", async () => {
  const fixture = structuredClone(snapshot);
  const document = (fixture.catalog.documents as JsonObject[])[0];
  const soldierRows = document.soldiers as Record<string, JsonObject>;
  soldierRows.test_rifle__test_grenadier = structuredClone(soldierRows.test_grenadier);
  (soldierRows.test_rifle__test_grenadier.mounts as JsonObject[])[1].weapons = ["rifle"];
  (document.units as Record<string, JsonObject>).test_rifle.body = {
    squad: { slots: ["test_rifleman", "test_rifle__test_grenadier"] },
  };
  fixture.documents.push({
    path: "local.json",
    value: {
      soldiers: {
        test_rifle__test_grenadier: {
          extends: "test_grenadier",
          mounts: [{ id: "grenade launcher", weapons: ["rifle"] }],
        },
      },
    },
  });
  const accepted = structuredClone(fixture.catalog);
  const acceptedDocument = (accepted.documents as JsonObject[])[0];
  delete (acceptedDocument.soldiers as JsonObject).test_rifle__test_grenadier;
  (acceptedDocument.units as Record<string, JsonObject>).test_rifle.body = {
    squad: { slots: ["test_rifleman", "test_grenadier"] },
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) =>
      url.endsWith("/preview")
        ? respond({
            revision: fixture.revision,
            catalog: accepted,
            soldierIds: {
              '["soldiers","test_rifle__test_grenadier","test_rifle"]': "test_grenadier",
            },
            affectedUnits: ["test_rifle"],
            warnings: [],
            files: [],
          })
        : respond(fixture),
    ),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle Section" }));
  const mounts = screen.getByLabelText("test_rifle__test_grenadier Weapon mounts");
  fireEvent.click(
    within(mounts.closest(".me-field") as HTMLElement).getByRole("button", {
      name: "Restore inherited value",
    }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  const summary = within(await screen.findByRole("region", { name: "Save preview" })).getByRole(
    "listitem",
  );
  const after = summary.querySelector(".me-change-values strong")!.textContent!.split(" → ")[1];
  expect(JSON.parse(after!)).toMatchObject([
    { id: "rifles", weapons: ["rifle"] },
    { id: "grenade launcher", weapons: ["grenade"] },
  ]);
});

it("returning from authoring uses the app menu without replacing the document", async () => {
  vi.stubGlobal("fetch", async () => respond(snapshot));
  render(
    <MemoryRouter initialEntries={["/mechanics"]}>
      <Routes>
        <Route path="/mechanics" element={<MechanicsEditor />} />
        <Route path="/" element={<div>Retained app menu</div>} />
      </Routes>
    </MemoryRouter>,
  );
  fireEvent.click(await screen.findByRole("link", { name: "← Developer menu" }));
  expect(screen.getByText("Retained app menu")).toBeTruthy();
  expect(screen.queryByRole("heading", { name: "Mechanics" })).toBeNull();
});

it("shows the neutral spotting value in preview and after saving a cleared input", async () => {
  const fixture = structuredClone(snapshot);
  const accepted = structuredClone(fixture.catalog);
  const rows = (accepted.documents as JsonObject[])[0].units as Record<string, JsonObject>;
  delete (rows.test_recon.sensors as JsonObject).spotting;
  const unit = (accepted.units as JsonObject[]).find((u) => u.id === "test_recon")!;
  delete (unit.sensors as JsonObject).spotting;
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => {
      if (url.endsWith("/preview"))
        return respond({
          revision: fixture.revision,
          catalog: accepted,
          soldierIds: {},
          affectedUnits: ["test_recon"],
          warnings: [],
          files: [],
        });
      if (url.endsWith("/save"))
        return respond({ ...fixture, revision: "saved", catalog: accepted });
      return respond(fixture);
    }),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.change(await screen.findByRole("searchbox"), { target: { value: "test_recon" } });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Recon Team" }));
  const input = screen.getByLabelText("test_recon Spotting");
  expect((input as HTMLInputElement).value).toBe("2");
  fireEvent.change(input, { target: { value: "" } });
  fireEvent.blur(input);
  expect((input as HTMLInputElement).value).toBe("1");
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  const preview = await screen.findByRole("region", { name: "Save preview" });
  expect(within(preview).getByText("2 → 1")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Save authored JSON" }));
  await waitFor(() => expect(screen.queryByRole("region", { name: "Save preview" })).toBeNull());
  expect((screen.getByLabelText("test_recon Spotting") as HTMLInputElement).value).toBe("1");
});
