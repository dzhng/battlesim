// @vitest-environment node
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { expect, test } from "vitest";
import {
  CONTACT_LINE_ROWS,
  CONTACT_LINE_SECONDS,
  contactLines,
  type LineContact,
} from "../src/battle/present/contactLines";
import { liveContactsNewestFirst } from "../src/battle/input/contactFocus";

const HZ = 20;
const WINDOW = CONTACT_LINE_SECONDS * HZ;
const units = { type: (kind: string) => ({ name: `type ${kind}` }) } as unknown as UnitCatalog;

const contact = (c: Partial<LineContact> & { id: number }): LineContact => ({
  source: "firing",
  kind: null,
  evidenceTick: 0,
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
  const lines = contactLines(units, contacts, 300, HZ);
  const reachable = liveContactsNewestFirst(contacts).map((c) => c.id);
  expect(lines.map((l) => l.key)).toEqual([2, 1]);
  expect(lines.every((l) => reachable.includes(l.key))).toBe(true);
  expect(contactLines(units, contacts, 250 + WINDOW, HZ).map((l) => l.key)).toEqual([2]);
});

test("only the newest few contacts show", () => {
  const many = Array.from({ length: CONTACT_LINE_ROWS + 2 }, (_, i) =>
    contact({ id: i + 1, evidenceTick: i }),
  );
  const lines = contactLines(units, many, many.length, HZ);
  expect(lines).toHaveLength(CONTACT_LINE_ROWS);
  expect(lines[0].key).toBe(many.length);
});

test("a line names an identified contact's type and nothing for an unidentified one", () => {
  const [known, unknown] = contactLines(
    units,
    [contact({ id: 1, kind: "t72", evidenceTick: 2 }), contact({ id: 2, evidenceTick: 1 })],
    2,
    HZ,
  );
  expect(known.text).toContain("type t72");
  expect(unknown.text).not.toContain("type");
});
