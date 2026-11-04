// @vitest-environment node
// The unseen look's seam to the page: the presentation crate checks a
// `presentation.fog` style and resolves it into the `FogLook` record
// (`crates/presentation/src/fog.rs` tests the rules); the page hands it to
// the fog mask passes as it is. The GPU half (seen pixels untouched, every
// material path styled) runs in the `fog-look` scene.
import { presentationSettings } from "@web/presentationSettings";
import { beforeAll, expect, test } from "vitest";
import { type FogPresentation } from "@packages/battle-renderer/src/fog";
import { FOG_LOOK_WORDS, FogLookAt } from "@packages/battle-renderer/src/records.gen";
import { loadPresentation, resolveFogLook } from "@web/battle/sim/presentation";

const fixture = presentationSettings.fog as unknown as FogPresentation;

beforeAll(() => loadPresentation());

test("every style the fixture names resolves into the look's record", () => {
  expect(Object.keys(fixture.styles)).toContain(fixture.style);
  for (const style of Object.values(fixture.styles)) {
    const look = resolveFogLook(style);
    expect(look.length).toBe(FOG_LOOK_WORDS);
    expect(look[FogLookAt.rim_width_px]).toBe(style.rim.width_px);
  }
});

test("a style out of range is refused by the field's name", () => {
  const style = structuredClone(fixture.styles[fixture.style]);
  expect(() => resolveFogLook({ ...style, rim: { ...style.rim, alpha: 1.5 } })).toThrow(
    /rim.alpha/,
  );
});
