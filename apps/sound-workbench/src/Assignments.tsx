import { SOUNDS } from "../../../packages/battle-audio/src/synth";
import { Choices } from "./controls";
import type { EditorProps } from "./editorProps";
export function UnitAssignments({
  snapshot,
  draft,
  edit,
  play,
  search,
  setSearch,
  unitId,
  setUnitId,
}: EditorProps & {
  search: string;
  setSearch(value: string): void;
  unitId: string;
  setUnitId(value: string): void;
}) {
  const matches = (text: string) => text.toLowerCase().includes(search.toLowerCase());
  return (
    <div className="sw-layout">
      <aside className="sw-browser">
        <label>
          Find unit type
          <input
            aria-label="Find unit type"
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
        {snapshot.units
          .filter((unit) => matches(`${unit.id} ${unit.name}`))
          .map((unit) => (
            <button
              className="sw-library-row"
              aria-label={unit.name}
              aria-pressed={unitId === unit.id}
              key={unit.id}
              onClick={() => setUnitId(unit.id)}
            >
              <span>{unit.name}</span>
              <small>
                {unit.id} ·{" "}
                {unit.mounts.length ? `${unit.mounts.length} mount choices` : "no firing mounts"}
              </small>
            </button>
          ))}
      </aside>
      <section className="sw-inspector sw-detail">
        {(() => {
          const unit = snapshot.units.find((u) => u.id === unitId);
          if (!unit)
            return (
              <>
                <h2>Choose an exact unit type</h2>
                <p>
                  Variants have independent choices. Equivalent named physical mounts share one
                  assignment.
                </p>
              </>
            );
          return (
            <>
              <p className="sw-eyebrow">EXACT UNIT TYPE · {unit.id}</p>
              <h2>{unit.name}</h2>
              <p>
                Launch choices follow the mount. Cannon mounts use one choice across their
                ammunition types.
              </p>
              {unit.mounts.map((mount) => {
                const kind = mount.weapons[0];
                const override = draft.units[unit.id]?.[mount.name];
                const global = draft.defaults[kind] ?? draft.defaults.default;
                const base = snapshot.firing[kind] ?? snapshot.firing.default;
                const choice = override ?? global ?? { near: base.near, far: base.far, gain: 1 };
                return (
                  <div className="sw-assignment" key={mount.name}>
                    <h3>{mount.name}</h3>
                    <p>
                      {mount.weapons.join(" / ")} ·{" "}
                      {override
                        ? "unit override"
                        : global
                          ? "global firing default"
                          : "original firing fallback"}
                    </p>
                    {mount.count > 1 && <p>{mount.count} physical mounts share this choice</p>}
                    <Choices
                      name={mount.name}
                      choice={choice}
                      catalog={draft}
                      onChange={(value) =>
                        edit((c) => {
                          (c.units[unit.id] ??= {})[mount.name] = value;
                        })
                      }
                      play={(id, gain) => void play("sound", id, 0, gain)}
                    />
                    <button
                      disabled={!override}
                      aria-label={`Restore ${mount.name} fallback`}
                      onClick={() =>
                        edit((c) => {
                          delete c.units[unit.id][mount.name];
                          if (!Object.keys(c.units[unit.id]).length) delete c.units[unit.id];
                        })
                      }
                    >
                      Restore fallback
                    </button>
                  </div>
                );
              })}
              {!unit.mounts.length && (
                <p>
                  This type has no firing mount. Its movement and environmental slots remain under
                  Defaults & effects.
                </p>
              )}
            </>
          );
        })()}
      </section>
    </div>
  );
}

export function GlobalAssignments({ snapshot, draft, edit, play }: EditorProps) {
  return (
    <section className="sw-detail sw-global">
      <h2>Global firing defaults</h2>
      <p>Used when an exact unit mount has no override.</p>
      {Object.entries(snapshot.firing).map(([kind, base]) => (
        <div className="sw-assignment" key={kind}>
          <h3>{kind}</h3>
          <Choices
            name={kind}
            choice={draft.defaults[kind] ?? { near: base.near, far: base.far, gain: 1 }}
            catalog={draft}
            onChange={(value) =>
              edit((c) => {
                c.defaults[kind] = value;
              })
            }
            play={(id, gain) => void play("sound", id, 0, gain)}
          />
          <button
            disabled={!draft.defaults[kind]}
            onClick={() =>
              edit((c) => {
                delete c.defaults[kind];
              })
            }
          >
            Restore {kind} firing fallback
          </button>
        </div>
      ))}
      <h2>Round / material impacts</h2>
      <p>
        Each round may replace the sound for the material it actually hits. The default row applies
        to every unassigned round.
      </p>
      {snapshot.materials.map((material) => (
        <details key={material}>
          <summary>{material}</summary>
          {["default", ...snapshot.rounds].map((round) => (
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
        Replace an implicit baseline slot. Explicit unit and impact selections keep their recipe.
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
