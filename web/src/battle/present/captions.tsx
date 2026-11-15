/** The status lines above the army deck: one per fresh contact, the patches N
 *  steps through. A line names what the side knows of the contact (its type
 *  once identified, and how it was found) and lasts a few seconds after the
 *  contact's latest evidence, while the contact is live. A heard sound alone
 *  is never a line: it marks nothing on the map, so the player could not act
 *  on it (`battle-audio` still plays it). */
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { ContactView } from "../sim/observationView";
import { contactFocusTargets } from "../input/contactFocus";

/** Rows shown at most. */
export const CAPTION_ROWS = 3;
/** A row lasts this long after its contact's latest evidence (seconds). */
export const CAPTION_SECONDS = 5;

export interface CaptionLine {
  /** The contact's handle. */
  key: number;
  text: string;
}

/** The contact fields a caption reads. */
export type CaptionContact = Pick<ContactView, "id" | "source" | "kind" | "evidenceTick"> & {
  center: readonly number[];
  retiring: boolean;
};

/** What one contact's line says. */
export function describeContact(units: UnitCatalog, c: CaptionContact): string {
  if (c.source === "radar") return "Radar contact";
  const name = c.kind ? units.type(c.kind).name : "Unknown";
  return c.source === "last_seen" ? `${name}, last seen` : `${name}, firing`;
}

/** The lines at `tick`: live contacts with recent evidence, newest first, in
 *  N's order, at most CAPTION_ROWS. */
export function contactCaptions(
  units: UnitCatalog,
  contacts: readonly CaptionContact[],
  tick: number,
  tickHz: number,
): CaptionLine[] {
  const byId = new Map(contacts.map((c) => [c.id, c]));
  return contactFocusTargets(contacts)
    .filter((t) => tick - t.evidenceTick < CAPTION_SECONDS * tickHz)
    .slice(0, CAPTION_ROWS)
    .map((t) => ({ key: t.id, text: describeContact(units, byId.get(t.id)!) }));
}

/** Fresh contacts, newest first: nothing at all while there are none. A lab
 *  can list them in its own style. */
export function CaptionList({
  lines,
  className = "hud-subtitles",
}: {
  lines: readonly CaptionLine[];
  className?: string;
}) {
  return (
    <ul className={className} data-testid="captions">
      {lines.map((c) => (
        <li key={c.key}>{c.text}</li>
      ))}
    </ul>
  );
}
