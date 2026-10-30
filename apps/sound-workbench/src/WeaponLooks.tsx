import { useState } from "react";
import type { NearFar } from "../../../packages/battle-audio/src/audioMix";
import type { EditorProps } from "./editorProps";
import type { LookSound } from "./protocol";
import { NumberField } from "./controls";

export function WeaponLooks({
  snapshot,
  draft,
  play,
  sounds,
  edit,
  stop,
}: Pick<EditorProps, "snapshot" | "draft" | "play"> & {
  sounds: Record<string, LookSound>;
  edit(change: (sounds: Record<string, LookSound>) => void): void;
  stop(): void;
}) {
  const [selected, setSelected] = useState(Object.keys(sounds)[0]);
  const sound = sounds[selected];
  if (!sound) return <p>No weapon looks in this generation.</p>;
  const update = (change: (sound: LookSound) => void) => edit((looks) => change(looks[selected]));
  const recipe = (label: string, value: string, change: (value: string) => void) => (
    <label>
      {label}
      <select
        aria-label={label}
        title={value}
        value={value}
        onChange={(event) => change(event.target.value)}
      >
        {Object.keys(draft.sounds).map((id) => (
          <option key={id} value={id}>
            {id}
          </option>
        ))}
      </select>
      <small className="sw-look-recipe-label">{draft.sounds[value].label}</small>
    </label>
  );
  const report = (
    part: "shot" | "blast",
    label: string,
    defaults: Pick<NearFar, "near" | "far">,
  ) => {
    const row = sound[part];
    if (!row)
      return (
        <section className="sw-assignment">
          <h3>{label}</h3>
          <p>No {part} authored.</p>
        </section>
      );
    return (
      <section className="sw-assignment">
        <h3>{label}</h3>
        <div className="sw-controls">
          {(["near", "far"] as const).map((band) => (
            <div key={band} className="sw-look-recording">
              {recipe(`${label} ${band}`, row[band] || defaults[band], (value) =>
                update((s) => {
                  s[part]![band] = value;
                }),
              )}
              <button onClick={() => void play("sound", row[band] || defaults[band], 0, row.gain)}>
                Play {part} {band}
              </button>
            </div>
          ))}
          <NumberField
            label={`${label} gain`}
            value={row.gain}
            max={100}
            onChange={(value) =>
              update((s) => {
                s[part]!.gain = value;
              })
            }
          />
          <NumberField
            label={`${label} far distance (m)`}
            value={row.far_m}
            max={Number.MAX_VALUE}
            onChange={(value) =>
              update((s) => {
                s[part]!.far_m = value;
              })
            }
          />
        </div>
      </section>
    );
  };
  const motor = sound.motor;
  const impacts = { ...snapshot.weaponDefaults.impacts, ...sound.impacts };
  return (
    <section className="sw-detail sw-global" data-weapon-looks>
      <h2>Weapon looks</h2>
      <label>
        Weapon look
        <select
          aria-label="Weapon look"
          value={selected}
          onChange={(event) => {
            stop();
            setSelected(event.target.value);
          }}
        >
          {Object.keys(sounds).map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
      </label>
      {report("shot", "Shot", { near: "", far: "" })}
      {report("blast", "Blast", snapshot.weaponDefaults.blast)}
      <section className="sw-assignment">
        <h3>Motor</h3>
        {motor ? (
          <div className="sw-controls">
            {recipe("Motor recording", motor.sound || snapshot.weaponDefaults.motor, (value) =>
              update((s) => {
                s.motor!.sound = value;
              }),
            )}
            <NumberField
              label="Motor gain"
              value={motor.gain}
              max={100}
              onChange={(value) =>
                update((s) => {
                  s.motor!.gain = value;
                })
              }
            />
            <button
              onClick={() =>
                void play("sound", motor.sound || snapshot.weaponDefaults.motor, 0, motor.gain)
              }
            >
              Play motor
            </button>
          </div>
        ) : (
          <p>No motor authored.</p>
        )}
      </section>
      <section className="sw-assignment sw-look-impacts">
        <h3>Impacts</h3>
        {sound.impact_scale !== undefined && (
          <NumberField
            label="Impact scale"
            value={sound.impact_scale}
            max={100}
            onChange={(value) =>
              update((s) => {
                s.impact_scale = value;
              })
            }
          />
        )}
        {Object.entries(impacts).map(([hit, id]) => (
          <div className="sw-effect-row" key={hit}>
            {recipe(`${hit.replaceAll("_", " ")} impact`, id, (value) =>
              update((s) => {
                s.impacts[hit] = value;
              }),
            )}
            <button
              onClick={() =>
                void play(
                  "sound",
                  id,
                  0,
                  (sound.impact_scale ?? 1) *
                    (hit === "ricochet" ? snapshot.ricochet.gain : snapshot.impacts[hit].gain),
                )
              }
            >
              Play {hit.replaceAll("_", " ")} impact
            </button>
          </div>
        ))}
      </section>
    </section>
  );
}
