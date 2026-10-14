// Radar contacts (slice 04): an EA-18G (SEAD) holds over the west edge
// while red's air defence keeps its fire. Every emitting AA, in the open,
// in the wood and behind the tower alike, is published as a radar contact:
// an area round (never on) the emitter, with a bearing and range from the
// SEAD jet quantized to 5° and 100 m, each within one step of the truth, never identified and never clearing
// fog; the silent short gun is no contact; the contacts refresh under the
// same ids. With an ordinary fighter alone there is no radar contact.
//
// That ground AA still needs line of sight to fire is the air-defense
// checkpoint's (its hidden gun).
import { gpuWarnings, aim } from "./_lab.mjs";
import { encounter, openAir, play, own, shoot, visibleCells, cardText } from "./_air.mjs";
import { unitType } from "./_units.mjs";

const sead = encounter("radar-sead");
const detector = sead.units.find((u) => unitType(u.kind).capabilities.air?.sead);
const emitters = sead.units.filter(
  (u) => u.side === "red" && unitType(u.kind).capabilities.radar?.emits,
);

export async function run(ctx) {
  let page = await openAir(ctx, "sead");
  let warnings = gpuWarnings(page);
  let fog = 0;
  let identified = 0;
  let o = await play(page, 60, (o) => {
    fog = Math.max(fog, visibleCells(o));
    identified += o.identified.length;
    return false;
  });
  const radar = o.contacts.filter((c) => c.source === "radar");
  ctx.check(
    "one radar contact per emitting AA, none for the silent gun",
    radar.length === emitters.length,
    JSON.stringify(radar.map((c) => c.center)),
  );
  for (const e of emitters) {
    const c = radar.find(
      (c) => Math.hypot(c.center[0] - e.position[0], c.center[1] - e.position[1]) <= c.radius,
    );
    const dx = e.position[0] - detector.position[0];
    const dy = e.position[1] - detector.position[1];
    // The published bearing is in [0, 360), counter-clockwise from east.
    const bearing = ((((Math.atan2(dy, dx) * 180) / Math.PI) % 360) + 360) % 360;
    const range = Math.hypot(dx, dy);
    ctx.check(
      `${e.kind}: an approximate area round it, with its quantized bearing and range from the SEAD jet`,
      !!c &&
        (c.center[0] !== e.position[0] || c.center[1] !== e.position[1]) &&
        c.kind === null &&
        c.bearingDeg >= 0 &&
        c.bearingDeg < 360 &&
        c.bearingDeg % 5 === 0 &&
        c.rangeM % 100 === 0 &&
        Math.abs(((c.bearingDeg - bearing + 540) % 360) - 180) <= 5 &&
        Math.abs(c.rangeM - range) <= 100,
      JSON.stringify({ c, bearing, range }),
    );
  }
  ctx.check("radar never identifies an emitter", identified === 0, `${identified}`);
  ctx.check("radar never clears blue's fog", fog === 0, `${fog} cells`);
  // Close enough for the cards to open.
  await aim(page, radar[0].center, { distance: 140, pitch: 0.8 });
  const text = await cardText(page, "");
  ctx.check(
    "the contact's card reads RADAR CONTACT with its bearing and range",
    !!text && text.includes("RADAR CONTACT") && /BEARING .*RANGE/.test(text),
    text ?? "no card",
  );
  await shoot(ctx, page, "radar-contacts");
  o = await play(page, 90);
  const later = o.contacts.filter((c) => c.source === "radar");
  ctx.check(
    "the contacts refresh under the same ids",
    JSON.stringify(later.map((c) => c.id).sort()) ===
      JSON.stringify(radar.map((c) => c.id).sort()) &&
      later.every((c) => c.evidenceTick > radar[0].evidenceTick),
    JSON.stringify(later.map((c) => [c.id, c.evidenceTick])),
  );
  ctx.check("sead: no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
  await page.close();

  page = await openAir(ctx, "fighter");
  warnings = gpuWarnings(page);
  let any = 0;
  o = await play(page, 120, (o) => {
    any += o.contacts.filter((c) => c.source === "radar").length;
    return false;
  });
  ctx.check(
    "an ordinary fighter detects no radar signature",
    any === 0 && own(o, "f_16c_block_50").length === 1,
    `${any}`,
  );
  ctx.check("fighter: no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
