import { SOUNDS } from "../../../packages/battle-audio/src/synth";
import { resolveEffect, resolveShot } from "../../../packages/battle-audio/src/catalog";
import { inheritRows } from "../../../packages/renderer-core/src/kindTable";
import { Choices } from "./controls";
import type { EditorProps } from "./editorProps";

/** Each vehicle class's loops, as the battle plays them: read only here,
 *  since the classes and their rows live in `presentation.audio.vehicles`. */
export function VehicleClasses({ snapshot, draft, play }: EditorProps) {
  return (
    <section className="sw-detail sw-global">
      <h2>Vehicle classes</h2>
      <p>
        A vehicle sounds as its class: how it moves, its hull&apos;s weight and whether it hauls
        supply. Classes and their levels live in the game&apos;s presentation; replace a loop for
        every class under Defaults &amp; effects.
      </p>
      {Object.entries(snapshot.vehicles).map(([cls, row]) => (
        <div className="sw-assignment" key={cls}>
          <h3>{cls}</h3>
          {(
            [
              ["engine", row.engine],
              ["running gear", row.running],
              ["turret", row.turret],
              ["reverse", row.reverse],
            ] as const
          ).map(([slot, baseline]) => {
            if (!baseline) return null;
            const sound = resolveEffect(draft, baseline);
            return (
              <div className="sw-effect-row" key={slot}>
                <span>
                  {slot} · {draft.sounds[sound]?.label ?? sound}
                </span>
                <button
                  aria-label={`Play ${cls} ${slot}`}
                  onClick={() => void play("sound", sound)}
                >
                  Play
                </button>
              </div>
            );
          })}
        </div>
      ))}
    </section>
  );
}

export function GlobalAssignments({ snapshot, draft, edit, play }: EditorProps) {
  const rows = Object.keys(snapshot.weapons);
  const bases = inheritRows(snapshot.firing, snapshot.weapons);
  // The row each weapon's choice comes from: its own, or its nearest
  // ancestor's that has one.
  const owners = inheritRows(
    Object.fromEntries(Object.keys(draft.defaults).map((kind) => [kind, kind])),
    snapshot.weapons,
  );
  const heard = { ...draft, defaults: inheritRows(draft.defaults, snapshot.weapons) };
  return (
    <section className="sw-detail sw-global">
      <h2>Firing by weapon row</h2>
      <p>
        A weapon row without a choice of its own fires its nearest ancestor&apos;s, else the default
        row&apos;s.
      </p>
      {["default", ...rows].map((kind) => {
        const own = Object.hasOwn(draft.defaults, kind);
        const owner = owners[kind];
        const { near, far, gain } = resolveShot(
          heard,
          { ...(bases[kind] ?? bases.default), gain: 1 },
          kind,
        );
        return (
          <div className="sw-assignment" key={kind}>
            <h3>{kind}</h3>
            <p>
              {own
                ? "own choice"
                : owner
                  ? `inherited from ${owner}`
                  : draft.defaults.default
                    ? "default row"
                    : "implicit firing fallback"}
            </p>
            <Choices
              name={kind}
              choice={{ near, far, gain }}
              catalog={draft}
              onChange={(value) =>
                edit((c) => {
                  c.defaults[kind] = value;
                })
              }
              play={(id, gain) => void play("sound", id, 0, gain)}
            />
            <button
              disabled={!own}
              onClick={() =>
                edit((c) => {
                  delete c.defaults[kind];
                })
              }
            >
              Restore {kind} firing fallback
            </button>
          </div>
        );
      })}
      <h2>Round / material impacts</h2>
      <p>
        Each round may replace the sound for the material it actually hits. The default row applies
        to every unassigned round.
      </p>
      {snapshot.materials.map((material) => (
        <details key={material}>
          <summary>{material}</summary>
          {["default", ...rows].map((round) => (
            <div className="sw-effect-row" key={round}>
              <label>
                {round}
                <select
                  aria-label={`${material} ${round} impact`}
                  value={draft.impacts[material]?.[round] ?? ""}
                  onChange={(e) =>
                    edit((c) => {
                      if (e.target.value) (c.impacts[material] ??= {})[round] = e.target.value;
                      else if (c.impacts[material]) {
                        delete c.impacts[material][round];
                        if (!Object.keys(c.impacts[material]).length) delete c.impacts[material];
                      }
                    })
                  }
                >
                  <option value="">Original / material fallback</option>
                  {Object.entries(draft.sounds)
                    .filter(([, sound]) => !sound.loop)
                    .map(([id, sound]) => (
                      <option key={id} value={id}>
                        {sound.label}
                      </option>
                    ))}
                </select>
              </label>
              <button
                disabled={!draft.impacts[material]?.[round]}
                onClick={() => void play("sound", draft.impacts[material][round])}
              >
                Play impact
              </button>
            </div>
          ))}
        </details>
      ))}
      <h2>Effects, flight and movement</h2>
      <p>
        Replace an implicit baseline slot. Explicit weapon and impact selections keep their recipe.
      </p>
      {Object.entries(SOUNDS).map(([slot, sound]) => (
        <div className="sw-effect-row" key={slot}>
          <label>
            {slot}
            <select
              aria-label={`${slot} effect`}
              value={draft.effects[slot] ?? ""}
              onChange={(e) =>
                edit((c) => {
                  if (e.target.value) c.effects[slot] = e.target.value;
                  else delete c.effects[slot];
                })
              }
            >
              <option value="">Original synthesis</option>
              {Object.entries(draft.sounds)
                .filter(([, recipe]) => recipe.loop === sound.loop)
                .map(([id, recipe]) => (
                  <option key={id} value={id}>
                    {recipe.label}
                  </option>
                ))}
            </select>
          </label>
          <button onClick={() => void play("sound", draft.effects[slot] ?? slot)}>
            Play effect
          </button>
        </div>
      ))}
    </section>
  );
}
