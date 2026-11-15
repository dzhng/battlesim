// @vitest-environment node
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { expect, test } from "vitest";
import {
  CAPTION_ROWS,
  CAPTION_SECONDS,
  contactCaptions,
  type CaptionContact,
} from "../src/battle/present/captions";
import { contactFocusTargets } from "../src/battle/input/contactFocus";

const HZ = 20;
const WINDOW = CAPTION_SECONDS * HZ;
const units = { type: (kind: string) => ({ name: `type ${kind}` }) } as unknown as UnitCatalog;

const contact = (c: Partial<CaptionContact> & { id: number }): CaptionContact => ({
  source: "firing",
  kind: null,
  evidenceTick: 0,
  center: [c.id * 100, 0],
  retiring: false,
  ...c,
});

test("each line is a live contact N can reach, in N's order, while its evidence is fresh", () => {
  const contacts = [
    contact({ id: 1, evidenceTick: 250 }),
    contact({ id: 2, evidenceTick: 300 }),
    contact({ id: 3, evidenceTick: 200, retiring: true }),
    contact({ id: 4, evidenceTick: 300 - WINDOW }),
  ];
  const lines = contactCaptions(units, contacts, 300, HZ);
  const reachable = contactFocusTargets(contacts).map((t) => t.id);
  expect(lines.map((l) => l.key)).toEqual([2, 1]);
  expect(lines.every((l) => reachable.includes(l.key))).toBe(true);
  expect(contactCaptions(units, contacts, 250 + WINDOW, HZ).map((l) => l.key)).toEqual([2]);
});

test("only the newest few contacts show", () => {
  const many = Array.from({ length: CAPTION_ROWS + 2 }, (_, i) =>
    contact({ id: i + 1, evidenceTick: i }),
  );
  const lines = contactCaptions(units, many, many.length, HZ);
  expect(lines).toHaveLength(CAPTION_ROWS);
  expect(lines[0].key).toBe(many.length);
});

test("a line names an identified contact's type and nothing for an unidentified one", () => {
  const [known, unknown] = contactCaptions(
    units,
    [contact({ id: 1, kind: "t72", evidenceTick: 2 }), contact({ id: 2, evidenceTick: 1 })],
    2,
    HZ,
  );
  expect(known.text).toContain("type t72");
  expect(unknown.text).not.toContain("type");
});
