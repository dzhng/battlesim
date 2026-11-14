// The Armory's sheet of the open card (`unitSheet`): its name and price, its
// variants, then its body, movement, senses and what else it does, and last
// its weapons by mount with each ammunition kind beneath. Strictly what a
// player reads (U3): no class lists, no internal fields.
import type { ReactNode } from "react";
import type { Reach, UnitCard } from "@packages/scene-assets/src/units";
import { glyphIcon, hudIcon, weaponIcon } from "@packages/scene-assets/src/icons";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { Icon } from "@web/battle/present/icons";
import {
  SHEET_STATS,
  unitSheet,
  type SheetAmmo,
  type SheetStat,
  type UnitSheet,
} from "@web/battle/present/unitSheet";

/** U7's sentence, word for word. */
const FIRES_REGARDLESS = "Fires at anything it sees, even what it can't hurt";

/** Each band's name, in the sheet's capitals; its range keeps its "m". */
const BANDS: Record<Reach["layer"], string> = {
  ground: "GROUND",
  low_air: "LOW AIR",
  high_air: "HIGH AIR",
};
/** The kinds of unit a band's class limit names (`Reach.only`). */
const KINDS: Record<NonNullable<Reach["only"]>[number], string> = {
  infantry: "infantry",
  vehicle: "vehicles",
  air_rotor: "helicopters",
  air_fixed_wing: "jets",
};

const figure = (value: number, digits: number) =>
  value.toLocaleString("en-US", { maximumFractionDigits: digits });

function Stats({ stats }: { stats: SheetStat[] }) {
  return (
    <dl className="armory-stats">
      {stats.map(({ key, value, scale }) => {
        const { label, unit, digits } = SHEET_STATS[key];
        return (
          <div key={key} className="armory-stat">
            <dt>{label}</dt>
            <dd className="armory-bar" aria-hidden>
              <span style={{ width: `${scale * 100}%` }} />
            </dd>
            <dd>
              {unit === "×" ? `×${figure(value, digits)}` : `${figure(value, digits)} ${unit}`}
            </dd>
          </div>
        );
      })}
    </dl>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="armory-section">
      <h4>{title}</h4>
      {children}
    </section>
  );
}

/** The body: a squad's men and their hit points, or a hull's strength and
 *  its armour, face by face. */
function Body({ sheet }: { sheet: UnitSheet }) {
  if (sheet.body === "squad") {
    const [low, high] = sheet.hpPerMan!;
    return (
      <Section title="Squad">
        <dl className="armory-facts">
          <div>
            <dt>Men</dt>
            <dd>{sheet.men}</dd>
          </div>
          <div>
            <dt>HP per man</dt>
            <dd>{low === high ? figure(low, 0) : `${figure(low, 0)}–${figure(high, 0)}`}</dd>
          </div>
        </dl>
      </Section>
    );
  }
  return (
    <Section title="Hull">
      <dl className="armory-facts">
        <div>
          <dt>Strength</dt>
          <dd>{figure(sheet.hp!, 0)} HP</dd>
        </div>
      </dl>
      <dl className="armory-armor" aria-label="Armour">
        {Object.entries(sheet.armor!).map(([face, value]) => (
          <div key={face}>
            <dd>{figure(value, 0)}</dd>
            <dt>{face}</dt>
          </div>
        ))}
      </dl>
    </Section>
  );
}

/** How far a kind reaches in each band it engages, the band's class limit
 *  where it has one, and whether it shoots down rounds there. */
function ReachLine({ reach }: { reach: Reach[] }) {
  return (
    <ul className="armory-reach" aria-label="Reach">
      {reach.map((band) => (
        <li key={band.layer}>
          {BANDS[band.layer]} {figure(band.range_m, 0)} m
          {band.only?.length ? <em>{band.only.map((k) => KINDS[k]).join(", ")} only</em> : null}
          {band.intercepts && <em>{band.only ? "intercepts only" : "intercepts"}</em>}
        </li>
      ))}
    </ul>
  );
}

/** A kind's marks: fires-regardless, and its rounds, a count or ∞. */
function AmmoMarks({ kind }: { kind: SheetAmmo }) {
  return (
    <>
      {kind.firesRegardless && (
        // Focusable, so the sentence reaches a keyboard as a pointer's hover.
        <span className="armory-mark" tabIndex={0} aria-label={FIRES_REGARDLESS}>
          <Icon path={hudIcon("fires_regardless")} />
          <span className="ro-cmd-tooltip" role="tooltip">
            {FIRES_REGARDLESS}
          </span>
        </span>
      )}
      <span className="armory-ammo-count" aria-label="Ammunition">
        {kind.ammo === "unlimited" ? <Icon path={glyphIcon("unlimited")} /> : kind.ammo}
        <small>rds</small>
      </span>
    </>
  );
}

/** One ammunition kind on its mount's rail. A mount's only kind carries its
 *  marks on the mount's own head, and takes no name of its own. */
function Ammo({ kind, alone }: { kind: SheetAmmo; alone: boolean }) {
  return (
    <div className="armory-ammo">
      {!alone && (
        <div className="armory-ammo-head">
          <span className="armory-ammo-name">{kind.name}</span>
          <AmmoMarks kind={kind} />
        </div>
      )}
      <p className="armory-description">{kind.description}</p>
      <Stats stats={kind.stats} />
      <ReachLine reach={kind.reach} />
    </div>
  );
}

interface ArmorySheetProps {
  card: UnitCard;
  /** The open group's label, over the name. */
  group: string;
  /** The family's cards under the open faction: a chip each, when several. */
  variants: readonly UnitCard[];
  onVariant: (card: string) => void;
}

export function ArmorySheet({ card, group, variants, onVariant }: ArmorySheetProps) {
  const sheet = unitSheet(useSessionCatalog(), card);
  return (
    <aside className="hud-panel armory-sheet" aria-label={`${card.name} sheet`}>
      <header className="armory-sheet-head">
        <span className="armory-kicker">{group}</span>
        <h3>{card.roster.family_name}</h3>
        <span className={`hud-purchase-price${sheet.planned ? " unsold" : ""}`}>
          {card.cost} CR
        </span>
        <span className="armory-variant">{card.roster.variant}</span>
        {sheet.planned ? (
          <span className="armory-tag">Planned</span>
        ) : (
          card.disabled_reason !== null && <span className="armory-tag">Not yet on sale</span>
        )}
      </header>
      {variants.length > 1 && (
        <div className="armory-variants" role="radiogroup" aria-label="Variants">
          {variants.map((v) => (
            <button
              key={v.id}
              type="button"
              role="radio"
              className="hud-menu-choice"
              aria-checked={v.id === card.id}
              onClick={() => onVariant(v.id)}
            >
              {v.roster.variant}
            </button>
          ))}
        </div>
      )}
      {/* Most cards' description is their variant's name: shown once, above. */}
      {![card.roster.variant, card.roster.family_name, ""].includes(card.description) && (
        <p className="armory-description">{card.description}</p>
      )}
      {sheet.planned ? (
        <p className="armory-planned">
          {sheet.mounts.length > 0
            ? "Not yet in battle: these are its planned weapons."
            : "Not yet in battle."}
        </p>
      ) : (
        <>
          <Body sheet={sheet} />
          <Section title="Movement">
            <Stats stats={sheet.movement} />
          </Section>
          <Section title="Senses">
            <Stats stats={sheet.senses} />
          </Section>
          {sheet.capabilities.length > 0 && (
            <ul className="armory-capabilities" aria-label="Capabilities">
              {sheet.capabilities.map((phrase) => (
                <li key={phrase}>{phrase}</li>
              ))}
            </ul>
          )}
        </>
      )}
      {/* An unarmed unit has no weapons heading: its absence says so. */}
      {sheet.mounts.length > 0 && (
        <Section title="Weapons">
          <ul className="armory-mounts">
            {sheet.mounts.map((mount, k) => (
              <li key={k} className="armory-mount">
                <div className="armory-mount-head">
                  <Icon path={weaponIcon(mount.icon)} />
                  <strong>{mount.name}</strong>
                  {mount.kinds.length === 1 && <AmmoMarks kind={mount.kinds[0]} />}
                </div>
                {mount.kinds.map((kind) => (
                  <Ammo key={kind.weapon} kind={kind} alone={mount.kinds.length === 1} />
                ))}
              </li>
            ))}
          </ul>
        </Section>
      )}
    </aside>
  );
}
