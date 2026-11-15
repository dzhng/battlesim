/** The status lines above the army deck: one per fresh contact, the patches N
 *  steps through. A line names what the side knows of the contact (its type
 *  once identified, and how it was found) and lasts a few seconds after the
 *  contact's latest evidence, while the contact is live. A heard sound alone
 *  is never a line: it marks nothing on the map, so the player could not act
 *  on it (`battle-audio` still plays it). */
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { liveContactsNewestFirst } from "../input/contactFocus";
import type { PresentedContact } from "./contactPresentation";

/** Rows shown at most. */
export const CONTACT_LINE_ROWS = 3;
/** A row lasts this long after its contact's latest evidence (seconds). */
export const CONTACT_LINE_SECONDS = 5;

export interface ContactLine {
  /** The contact's handle. */
  key: number;
  text: string;
}

/** The contact fields a line reads. */
export type LineContact = Pick<
  PresentedContact,
  "id" | "source" | "kind" | "evidenceTick" | "retiring"
>;

/** What one contact's line says. */
function describeContact(units: UnitCatalog, c: LineContact): string {
  if (c.source === "radar") return "Radar contact";
  const name = c.kind ? units.type(c.kind).name : "Unknown";
  return c.source === "last_seen" ? `${name}, last seen` : `${name}, firing`;
}

/** The lines at `tick`: live contacts with recent evidence, newest first, in
 *  N's order, at most CONTACT_LINE_ROWS. */
export function contactLines(
  units: UnitCatalog,
  contacts: readonly LineContact[],
  tick: number,
  tickHz: number,
): ContactLine[] {
  return liveContactsNewestFirst(contacts)
    .filter((c) => tick - c.evidenceTick < CONTACT_LINE_SECONDS * tickHz)
    .slice(0, CONTACT_LINE_ROWS)
    .map((c) => ({ key: c.id, text: describeContact(units, c) }));
}

/** Fresh contacts, newest first: nothing at all while there are none. */
export function ContactLines({ lines }: { lines: readonly ContactLine[] }) {
  return (
    <ul className="hud-contact-lines" data-testid="contact-lines">
      {lines.map((c) => (
        <li key={c.key}>{c.text}</li>
      ))}
    </ul>
  );
}
