import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import MechanicsEditor from "../../apps/mechanics-editor/src/MechanicsEditor";
import catalog from "../../fixtures/catalog.json";
import infantry from "../../fixtures/units/generic/infantry.json";
import soldiers from "../../fixtures/units/generic/soldiers.json";
import tanks from "../../fixtures/units/generic/tanks.json";
import game from "../../fixtures/game.json";
import type { JsonObject, MechanicsSnapshot } from "../../apps/mechanics-editor/src/protocol";

const snapshot: MechanicsSnapshot = {
  revision: "test-one",
  catalog: catalog as unknown as JsonObject,
  documents: [
    { path: "fixtures/units/generic/infantry.json", value: infantry },
    { path: "fixtures/units/generic/soldiers.json", value: soldiers },
    { path: "fixtures/units/generic/tanks.json", value: tanks as unknown as JsonObject },
    { path: "fixtures/game.json", value: game },
  ],
};
const respond = (value: unknown) =>
  new Response(JSON.stringify(value), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

it("shows exact units by default and preserves an inline edit while filtering", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => respond(snapshot)),
  );
  render(<MechanicsEditor />);
  await screen.findByRole("heading", { name: "Mechanics" });
  expect(await screen.findByRole("button", { name: "Expand Rifle squad" })).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Rifle squad deployment cost"), {
    target: { value: "125" },
  });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "tank" } });
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
  render(<MechanicsEditor />);
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
  render(<MechanicsEditor />);
  const cost = await screen.findByLabelText("Rifle squad deployment cost");
  fireEvent.change(cost, { target: { value: "120" } });
  fireEvent.change(cost, { target: { value: "12e" } });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "tank" } });
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
        affectedUnits: ["rifle"],
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
  render(<MechanicsEditor />);
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
    changes: [{ section: "units", id: "rifle", path: ["cost"], value: 130 }],
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
  render(<MechanicsEditor />);
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
  render(<MechanicsEditor />);
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
  render(<MechanicsEditor />);
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
