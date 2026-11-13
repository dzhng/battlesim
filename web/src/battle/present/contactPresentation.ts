import type { ContactView, ObservationView } from "../sim/observationView";

/** What an observation reports of its side's contacts, and whose they are. */
export type ContactReport = Pick<ObservationView, "contacts" | "tick"> & {
  groundPatch: Pick<ObservationView["groundPatch"], "side">;
};

/** Visual memory only. Retired reports cannot be selected or commanded. */
export interface PresentedContact extends ContactView {
  opacity: number;
  retiring: boolean;
}

/** One fade shared by the ground glyph and its label, on battle time. */
export class ContactPresentation {
  private held = new Map<number, { contact: ContactView; removedAt: number | null }>();
  private tick = -1;
  private side: string | null = null;
  constructor(
    private readonly tickHz: number,
    private readonly fadeSeconds: number,
  ) {}

  /** The contacts `report` shows. A rewound battle, or another side's
   *  report (a lab switching sides), forgets what was remembered. */
  update({ contacts, tick, groundPatch: { side } }: ContactReport): PresentedContact[] {
    if (tick < this.tick || side !== this.side) this.held.clear();
    this.tick = tick;
    this.side = side;
    const live = new Set(contacts.map((c) => c.id));
    for (const contact of contacts) this.held.set(contact.id, { contact, removedAt: null });
    const out: PresentedContact[] = [];
    for (const [id, held] of this.held) {
      if (!live.has(id)) held.removedAt ??= tick;
      const opacity =
        held.removedAt === null
          ? 1
          : Math.max(0, 1 - (tick - held.removedAt) / (this.fadeSeconds * this.tickHz));
      if (opacity > 0) out.push({ ...held.contact, opacity, retiring: held.removedAt !== null });
      else this.held.delete(id);
    }
    return out;
  }
}
