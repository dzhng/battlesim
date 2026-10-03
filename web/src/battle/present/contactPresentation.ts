import type { ContactView } from "../sim/observation";

/** Visual memory only. Retired reports cannot be selected or commanded. */
export interface PresentedContact extends ContactView {
  opacity: number;
  retiring: boolean;
}

/** One fade shared by the ground glyph and its label, on battle time. */
export class ContactPresentation {
  private held = new Map<number, { contact: ContactView; removedAt: number | null }>();
  private tick = -1;
  constructor(
    private readonly tickHz: number,
    private readonly fadeSeconds: number,
  ) {}

  update(contacts: readonly ContactView[], tick: number): PresentedContact[] {
    if (tick < this.tick) this.held.clear();
    this.tick = tick;
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
