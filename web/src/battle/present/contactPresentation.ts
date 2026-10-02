import type { ContactView } from "../sim/observation";
import { contactOpacity } from "@packages/battle-renderer/src/contactGlyph";

/** Visual memory only. Retired reports cannot be selected or commanded. */
export interface PresentedContact extends ContactView {
  opacity: number;
  retiring: boolean;
}

/** One fade shared by the ground glyph and its label, on battle time. */
export class ContactPresentation {
  private held = new Map<number, { contact: PresentedContact; removedAt: number | null }>();
  private tick = -1;
  constructor(
    private readonly tickHz: number,
    private readonly fadeSeconds: number,
  ) {}

  update(contacts: readonly ContactView[], tick: number): PresentedContact[] {
    if (tick < this.tick) this.held.clear();
    this.tick = tick;
    const live = new Set(contacts.map((c) => c.id));
    for (const contact of contacts)
      this.held.set(contact.id, {
        contact: {
          ...contact,
          opacity: contactOpacity(contact, tick, this.fadeSeconds * this.tickHz),
          retiring: false,
        },
        removedAt: null,
      });
    const out: PresentedContact[] = [];
    for (const [id, held] of this.held) {
      if (!live.has(id)) {
        held.removedAt ??= tick;
        const fade = Math.max(0, 1 - (tick - held.removedAt) / (this.fadeSeconds * this.tickHz));
        const opacity = Math.min(
          contactOpacity(held.contact, tick, this.fadeSeconds * this.tickHz),
          held.contact.opacity * fade,
        );
        if (opacity > 0) out.push({ ...held.contact, opacity, retiring: true });
        else this.held.delete(id);
      } else if (held.contact.opacity > 0) out.push(held.contact);
      else this.held.delete(id);
    }
    return out;
  }
}
