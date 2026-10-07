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
    test_rifleman: { hp: 100, mounts: [{ name: "rifles", weapons: ["rifle"] }] },
    test_grenadier: { hp: 100, mounts: [{ name: "launcher", weapons: ["grenade"] }] },
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
  expect(await screen.findByRole("button", { name: "Expand Rifle squad" })).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Rifle squad deployment cost"), {
    target: { value: "125" },
  });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "test_tank" } });
  expect(screen.queryByRole("button", { name: "Expand Rifle squad" })).toBeNull();
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
  expect((screen.getByLabelText("Rifle squad deployment cost") as HTMLInputElement).value).toBe(
    "125",
  );
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    false,
  );
});

it("links editable landing spread and angular scatter and keeps range edits in battlefield units", async () => {
  const fixture = structuredClone(snapshot);
  const weapons = fixture.catalog.weapons as Record<string, Record<string, number>>;
  weapons.grenade.range_m = 300;
  weapons.grenade.scatter_mrad = 15;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(fixture)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
  fireEvent.click(screen.getByText("Grenade", { selector: "summary" }));
  const spread = screen.getByLabelText(
    "grenade Landing spread at maximum range",
  ) as HTMLInputElement;
  const raw = screen.getByLabelText(
    "grenade Landing spread at maximum range — Angular scatter · milliradians",
  ) as HTMLInputElement;
  expect(spread.value).toBe("4.5");
  fireEvent.change(screen.getByLabelText("grenade Maximum engagement range"), {
    target: { value: "150" },
  });
  expect(spread.value).toBe("4.5");
  expect(raw.value).toBe("30");
  fireEvent.change(raw, { target: { value: "20" } });
  expect(spread.value).toBe("3");
  fireEvent.change(spread, { target: { value: "6" } });
  expect(raw.value).toBe("40");
  fireEvent.change(spread, { target: { value: "6." } });
  expect(spread.value).toBe("6.");
});

it("retains invalid text through filtering and refuses preview until it is corrected", async () => {
  const fetcher = vi.fn(async () => respond(snapshot));
  vi.stubGlobal("fetch", fetcher);
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  const cost = await screen.findByLabelText("Rifle squad deployment cost");
  fireEvent.change(cost, { target: { value: "120" } });
  fireEvent.change(cost, { target: { value: "12e" } });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "test_tank" } });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
  expect((screen.getByLabelText("Rifle squad deployment cost") as HTMLInputElement).value).toBe(
    "12e",
  );
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    true,
  );
  expect(fetcher.mock.calls.length).toBe(1);
  fireEvent.change(screen.getByLabelText("Rifle squad deployment cost"), {
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
  fireEvent.change(await screen.findByLabelText("Rifle squad deployment cost"), {
    target: { value: "125" },
  });
  const save = screen.getByRole("button", { name: "Save authored JSON" });
  expect(save.hasAttribute("disabled")).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("heading", { name: "Preview authored changes" });
  expect(save.hasAttribute("disabled")).toBe(false);
  fireEvent.change(screen.getByLabelText("Rifle squad deployment cost"), {
    target: { value: "130" },
  });
  expect(save.hasAttribute("disabled")).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await waitFor(() => expect(save.hasAttribute("disabled")).toBe(false));
  fireEvent.click(save);
  await screen.findByRole("status");
  expect(published).toBe(true);
  expect((screen.getByLabelText("Rifle squad deployment cost") as HTMLInputElement).value).toBe(
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
  fireEvent.change(await screen.findByLabelText("Rifle squad deployment cost"), {
    target: { value: "125" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByText("Sources changed outside this editor");
  expect((screen.getByLabelText("Rifle squad deployment cost") as HTMLInputElement).value).toBe(
    "125",
  );
  fireEvent.click(screen.getByRole("button", { name: "Reload sources" }));
  expect((screen.getByLabelText("Rifle squad deployment cost") as HTMLInputElement).value).toBe(
    "125",
  );
  fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
  expect(screen.queryByRole("button", { name: "Discard draft and reload" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Reload sources" }));
  fireEvent.click(screen.getByRole("button", { name: "Discard draft and reload" }));
  await waitFor(() =>
    expect((screen.getByLabelText("Rifle squad deployment cost") as HTMLInputElement).value).toBe(
      "100",
    ),
  );
});

it("undoing a range edit preserves landing spread and clears the coupled draft", async () => {
  const fixture = structuredClone(snapshot);
  const weapons = fixture.catalog.weapons as Record<string, Record<string, number>>;
  weapons.grenade.range_m = 150;
  weapons.grenade.scatter_mrad = 30;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(fixture)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
  fireEvent.click(screen.getByText("Grenade", { selector: "summary" }));
  const range = screen.getByLabelText("grenade Maximum engagement range");
  fireEvent.change(range, { target: { value: "300" } });
  fireEvent.click(range.closest(".me-field")!.querySelector("button")!);
  expect((range as HTMLInputElement).value).toBe("150");
  expect(
    (screen.getByLabelText("grenade Landing spread at maximum range") as HTMLInputElement).value,
  ).toBe("4.5");
  expect(
    (
      screen.getByLabelText(
        "grenade Landing spread at maximum range — Angular scatter · milliradians",
      ) as HTMLInputElement
    ).value,
  ).toBe("30");
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    true,
  );
});

it("restoring inherited range keeps the currently edited landing spread", async () => {
  const fixture = structuredClone(snapshot);
  const weapons = fixture.catalog.weapons as Record<string, Record<string, number>>;
  weapons.grenade.range_m = 150;
  weapons.grenade.scatter_mrad = 30;
  fixture.documents = [
    {
      path: "game.json",
      value: {
        weapons: {
          parent: { range_m: 300 },
          grenade: { extends: "parent", range_m: 150, scatter_mrad: 30 },
        },
      },
    },
  ];
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(fixture)),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
  fireEvent.click(screen.getByText("Grenade", { selector: "summary" }));
  fireEvent.change(screen.getByLabelText("grenade Landing spread at maximum range"), {
    target: { value: "6" },
  });
  const range = screen.getByLabelText("grenade Maximum engagement range");
  fireEvent.click(range.closest(".me-field")!.querySelector("button")!);
  expect((range as HTMLInputElement).value).toBe("300");
  expect(
    (screen.getByLabelText("grenade Landing spread at maximum range") as HTMLInputElement).value,
  ).toBe("6");
  expect(
    (
      screen.getByLabelText(
        "grenade Landing spread at maximum range — Angular scatter · milliradians",
      ) as HTMLInputElement
    ).value,
  ).toBe("20");
});

it("discarding an optional parent clears its unfinished child text without clearing other edits", async () => {
  const fetcher = vi.fn(async () => respond(structuralSnapshot()));
  vi.stubGlobal("fetch", fetcher);
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
  fireEvent.click(screen.getByText("Rifle", { selector: "summary" }));
  fireEvent.change(screen.getByLabelText("rifle Cyclic firing rate"), { target: { value: "-" } });
  fireEvent.change(screen.getByLabelText("Rifle squad deployment cost"), {
    target: { value: "12e" },
  });
  fireEvent.change(screen.getByLabelText("rifle Magazine / belt"), { target: { value: "" } });
  expect(screen.queryByLabelText("rifle Cyclic firing rate")).toBeNull();
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    true,
  );
  expect((screen.getByLabelText("Rifle squad deployment cost") as HTMLInputElement).value).toBe(
    "12e",
  );
  fireEvent.change(screen.getByLabelText("Rifle squad deployment cost"), {
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
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
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
        { name: "temporary", weapons: ["rifle"], pivot_m: [0, 0, 0] },
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
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
  fireEvent.change(screen.getByLabelText("rifle Cyclic firing rate"), { target: { value: "-" } });
  const magazine = screen.getByLabelText("rifle Magazine / belt");
  fireEvent.click(magazine.closest(".me-field")!.querySelector("button")!);
  expect(screen.queryByLabelText("rifle Cyclic firing rate")).toBeNull();
  expect(screen.getByRole("button", { name: "Preview changes" }).hasAttribute("disabled")).toBe(
    false,
  );
});

it("previews old and new battlefield values with their raw units across coupled edits and restoration", async () => {
  const fixture = structuredClone(snapshot);
  const weapons = fixture.catalog.weapons as Record<string, JsonObject>;
  weapons.grenade.range_m = 150;
  weapons.grenade.scatter_mrad = 30;
  fixture.documents = [
    {
      path: "game.json",
      value: {
        weapons: {
          parent: { range_m: 300 },
          grenade: { extends: "parent", range_m: 150, scatter_mrad: 30 },
        },
      },
    },
  ];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) =>
      url.endsWith("/preview")
        ? respond({
            revision: fixture.revision,
            catalog: {
              ...fixture.catalog,
              weapons: {
                ...weapons,
                grenade: { ...weapons.grenade, range_m: 300, scatter_mrad: 20 },
              },
            },
            affectedUnits: ["test_rifle"],
            warnings: [],
            files: [],
          })
        : respond(fixture),
    ),
  );
  render(<MechanicsEditor />, { wrapper: MemoryRouter });
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
  fireEvent.change(screen.getByLabelText("grenade Landing spread at maximum range"), {
    target: { value: "6" },
  });
  const range = screen.getByLabelText("grenade Maximum engagement range");
  fireEvent.click(range.closest(".me-field")!.querySelector("button")!);
  fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
  await screen.findByRole("heading", { name: "Preview authored changes" });
  const summary = within(screen.getByRole("region", { name: "Save preview" }));
  const spreadSummary = summary
    .getAllByRole("listitem")
    .find((item) => item.textContent?.includes("Landing spread at maximum range"))!;
  expect(spreadSummary.textContent).toContain("4.5 m → 6 m");
  expect(spreadSummary.textContent).toContain("30 → 20");
  expect(spreadSummary.textContent).toContain("Angular scatter · milliradians");
  const rangeSummary = summary
    .getAllByRole("listitem")
    .find((item) => item.textContent?.includes("Maximum engagement range"))!;
  expect(rangeSummary.textContent).toContain("150 m → 300 m");
  expect(rangeSummary.textContent).toContain("Restore inheritance");
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
    fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
    const spread = screen.getByLabelText("grenade Landing spread at maximum range");
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
      mounts: [{ name: "rifles", weapons: ["rifle"], squad: false, special: false }],
    },
    test_rifle__test_rifleman: {
      hp: 100,
      mounts: [{ name: "rifles", weapons: ["rifle"], squad: false, special: true }],
    },
  };
  fixture.documents = [
    {
      path: "soldiers.json",
      value: {
        soldiers: {
          test_rifleman: {
            hp: 100,
            mounts: [{ name: "rifles", weapons: ["rifle"], squad: false }],
          },
          test_rifle__test_rifleman: {
            extends: "test_rifleman",
            mounts: [{ name: "rifles", special: true }],
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
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
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
      mounts: [{ name: "rifles", weapons: ["rifle"], squad: true, special: false }],
    },
  };
  (fixture.catalog.units as JsonObject[]).find((unit) => unit.id === "test_rifle")!.body = {
    squad: { slots: ["test_rifleman"] },
  };
  const accepted = structuredClone(fixture.catalog);
  (accepted.soldiers as JsonObject).test_rifle__test_rifleman = {
    hp: 100,
    mounts: [{ name: "rifles", weapons: ["rifle"], squad: false, special: false }],
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
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
  fireEvent.change(screen.getByLabelText("test_rifleman Weapon mounts"), {
    target: { value: JSON.stringify([{ name: "rifles", weapons: ["rifle"] }]) },
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
          mounts: [{ name: "grenade launcher", weapons: ["rifle"] }],
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
  fireEvent.click(await screen.findByRole("button", { name: "Expand Rifle squad" }));
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
    { name: "rifles", weapons: ["rifle"] },
    { name: "grenade launcher", weapons: ["grenade"] },
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
