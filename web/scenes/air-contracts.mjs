// The air-war contract seam (slice 01): the fielded types' resolved
// records, printed by the lab's panel from the session catalog, are what
// the simulation runs: each aircraft's published fuel and flares start at
// its record's capacity, a ground unit publishes neither, and every
// aircraft holds the air height over the ground.
import { gpuWarnings, obs } from "./_lab.mjs";
import { encounter, openAir, layerOf, surface, CRUISE, shoot, own } from "./_air.mjs";
import { unitType, weapons } from "./_units.mjs";

const fielded = encounter("contracts").units;
const kinds = [...new Set(fielded.map((u) => u.kind))];

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  const o = await obs(page);
  await shoot(ctx, page, "contracts");

  // The panel: one record per fielded type, its fields the catalog's.
  const records = await page.evaluate(() =>
    [...document.querySelectorAll("[data-testid=air-records] [data-kind]")].map((s) => ({
      kind: s.getAttribute("data-kind"),
      fields: Object.fromEntries(
        [...s.querySelectorAll(".lab-row > span[data-field]")].map((f) => [
          f.getAttribute("data-field"),
          f.textContent.slice(f.getAttribute("data-field").length + 1),
        ]),
      ),
      weapons: Object.fromEntries(
        [...s.querySelectorAll("[data-weapon]")].map((w) => [
          w.getAttribute("data-weapon"),
          w.querySelector("[data-field=targets]").textContent,
        ]),
      ),
    })),
  );
  ctx.check(
    "the panel prints one record per fielded type",
    JSON.stringify(records.map((r) => r.kind).sort()) === JSON.stringify([...kinds].sort()),
    JSON.stringify(records.map((r) => r.kind)),
  );
  for (const kind of kinds) {
    const t = unitType(kind);
    const rec = records.find((r) => r.kind === kind);
    const caps = t.capabilities;
    const radar = caps.air?.radar ?? caps.radar;
    const want = {
      layer: layerOf(kind),
      fuel: caps.air ? String(caps.air.fuel_capacity_s) : "—",
      aa_tier: caps.air_defense?.tier ?? "—",
      radar: radar ? `${radar.emits ? "emits" : "silent"}/${radar.detection_m}` : "—",
      sead: caps.air?.sead ? "yes" : "no",
    };
    const targets = Object.fromEntries(
      t.mounts
        .flatMap((m) => m.weapons)
        .map((w) => [w, Object.keys(weapons[w].targets ?? { ground: {} }).join("+")]),
    );
    ctx.check(
      `${kind}: its record reads the catalog's layer, fuel, tier, radar and weapon layers`,
      !!rec &&
        Object.entries(want).every(([k, v]) => rec.fields[k] === v) &&
        JSON.stringify(rec.weapons) === JSON.stringify(targets),
      JSON.stringify({ rec, want, targets }),
    );
  }

  // The simulation runs those records: nothing beyond the encounter, fuel
  // and flares at capacity, the air height over the ground.
  ctx.check(
    "the battle fields exactly the encounter's units",
    o.own.length === fielded.length,
    `${o.own.length}`,
  );
  for (const kind of kinds) {
    const caps = unitType(kind).capabilities;
    for (const u of own(o, kind)) {
      const lift = u.position[2] - (await surface(page, u.position));
      const flies = layerOf(kind) !== "ground";
      const ok =
        (caps.air
          ? u.fuelS !== null &&
            u.fuelS <= caps.air.fuel_capacity_s &&
            u.fuelS > caps.air.fuel_capacity_s - 2 &&
            (u.flares?.charges ?? 0) === (caps.air.flares?.capacity ?? 0)
          : u.fuelS === null && u.flares === null) && Math.abs(lift - (flies ? CRUISE : 0)) < 1.5;
      ctx.check(
        `${kind}: published fuel, flares and height follow its record`,
        ok,
        JSON.stringify({ fuel: u.fuelS, flares: u.flares, lift }),
      );
    }
  }
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
