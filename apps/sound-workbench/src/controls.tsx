import { useEffect, useState } from "react";
import type {
  ShotChoice,
  SoundCatalog,
  SoundRecipe,
} from "../../../packages/battle-audio/src/catalog";
export function NumberField({
  label,
  value,
  max = 2,
  onChange,
}: {
  label: string;
  value: number;
  max?: number;
  onChange(value: number): void;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => {
    if (Number.isFinite(value)) setText(String(value));
  }, [value]);
  return (
    <label>
      {label}
      <input
        type="number"
        min="0"
        max={max}
        step="0.01"
        required
        aria-label={label}
        value={text}
        onChange={(event) => {
          setText(event.target.value);
          onChange(event.target.value === "" ? NaN : Number(event.target.value));
        }}
      />
    </label>
  );
}

function RecipeSelect({
  label,
  catalog,
  value,
  allow,
  onChange,
}: {
  label: string;
  catalog: SoundCatalog;
  value: string;
  allow?: (sound: SoundRecipe) => boolean;
  onChange(value: string): void;
}) {
  return (
    <label>
      {label}
      <select aria-label={label} value={value} onChange={(e) => onChange(e.target.value)}>
        {Object.entries(catalog.sounds)
          .filter(([, sound]) => !allow || allow(sound))
          .map(([id, sound]) => (
            <option key={id} value={id}>
              {sound.label}
            </option>
          ))}
      </select>
    </label>
  );
}

export function Choices({
  name,
  choice,
  catalog,
  onChange,
  play,
}: {
  name: string;
  choice: ShotChoice;
  catalog: SoundCatalog;
  onChange(value: ShotChoice): void;
  play(id: string, gain?: number): void;
}) {
  const firing = (sound: SoundRecipe) =>
    !sound.loop &&
    sound.clips.every((clip) => ["shot", "launch"].includes(catalog.clips[clip]?.role));
  return (
    <div className="sw-choices">
      <RecipeSelect
        label={`${name} near`}
        catalog={catalog}
        value={choice.near}
        allow={firing}
        onChange={(near) => onChange({ ...choice, near })}
      />
      <button onClick={() => play(choice.near, choice.gain)} aria-label={`Play ${name} near`}>
        Play
      </button>
      <RecipeSelect
        label={`${name} far`}
        catalog={catalog}
        value={choice.far}
        allow={firing}
        onChange={(far) => onChange({ ...choice, far })}
      />
      <button onClick={() => play(choice.far, choice.gain)} aria-label={`Play ${name} far`}>
        Play
      </button>
      <NumberField
        label={`${name} gain`}
        value={choice.gain}
        onChange={(gain) => onChange({ ...choice, gain })}
      />
    </div>
  );
}
