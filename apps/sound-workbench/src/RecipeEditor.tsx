import { useEffect, useState } from "react";
import { SOUNDS } from "../../../packages/battle-audio/src/synth";
import type { SoundCatalog, SoundRecipe } from "../../../packages/battle-audio/src/catalog";
import { NumberField } from "./controls";
export function RecipeEditor({
  id,
  catalog,
  change,
  clone,
  play,
}: {
  id: string;
  catalog: SoundCatalog;
  change(recipe: SoundRecipe): void;
  clone(): void;
  play(variant: number): void;
}) {
  const recipe = catalog.sounds[id];
  const baseline = Object.hasOwn(SOUNDS, id);
  const [filter, setFilter] = useState("");
  const [variant, setVariant] = useState(0);
  useEffect(
    () => setVariant((value) => Math.min(value, Math.max(0, recipe.clips.length - 1))),
    [recipe.clips.length],
  );
  const clips = Object.entries(catalog.clips).filter(
    ([, clip]) =>
      clip.loop === recipe.loop &&
      `${clip.label} ${clip.category} ${clip.role}`.toLowerCase().includes(filter.toLowerCase()),
  );
  return (
    <section className="sw-detail">
      <p className="sw-eyebrow">
        {baseline ? "SYNTHESIS BASELINE" : "SOUND RECIPE"} · {recipe.loop ? "LOOP" : "ONE SHOT"}
      </p>
      <h2>{recipe.label}</h2>
      <p className="sw-id">{id}</p>
      <div className="sw-actions">
        <button onClick={() => play(variant)}>Play recipe</button>
        <label>
          Variation
          <select
            aria-label="Variation"
            value={variant}
            onChange={(event) => setVariant(Number(event.target.value))}
          >
            {Array.from({ length: Math.max(1, recipe.clips.length) }, (_, i) => (
              <option key={i} value={i}>
                {i + 1}
                {recipe.clips[i] ? ` · ${catalog.clips[recipe.clips[i]].label}` : " · synthesis"}
              </option>
            ))}
          </select>
        </label>
        <button onClick={clone}>Clone recipe</button>
      </div>
      {baseline ? (
        <p>This baseline keeps its original sound. Clone it to build an editable recording mix.</p>
      ) : (
        <>
          <label>
            Recipe label
            <input
              aria-label="Recipe label"
              value={recipe.label}
              onChange={(e) => change({ ...recipe, label: e.target.value })}
            />
          </label>
          <div className="sw-controls">
            <NumberField
              label="Recording / recipe gain"
              value={recipe.gain}
              onChange={(gain) => change({ ...recipe, gain })}
            />
            <label>
              Synthesis support
              <select
                aria-label="Synthesis support"
                value={recipe.synth ?? ""}
                onChange={(e) => change({ ...recipe, synth: e.target.value || null })}
              >
                <option value="">None</option>
                {Object.keys(SOUNDS)
                  .filter((name) => SOUNDS[name].loop === recipe.loop)
                  .map((name) => (
                    <option key={name} value={name}>
                      {name}
                    </option>
                  ))}
              </select>
            </label>
            <NumberField
              label="Synthesis level"
              max={1}
              value={recipe.synth_gain}
              onChange={(synth_gain) => change({ ...recipe, synth_gain })}
            />
          </div>
          <p>
            Clips are alternatives, one per variation. Synthesis adds support to each alternative.
          </p>
          <label>
            Find clip alternatives
            <input
              type="search"
              aria-label="Find clip alternatives"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
            />
          </label>
          <div className="sw-alternatives">
            {clips.map(([clipId, clip]) => (
              <label key={clipId}>
                <input
                  type="checkbox"
                  checked={recipe.clips.includes(clipId)}
                  onChange={(e) =>
                    change({
                      ...recipe,
                      clips: e.target.checked
                        ? [...recipe.clips, clipId]
                        : recipe.clips.filter((value) => value !== clipId),
                    })
                  }
                />
                <span>
                  {clip.label}
                  <small>
                    {clip.role} · {(clip.frames / clip.sample_rate).toFixed(2)} s
                  </small>
                </span>
              </label>
            ))}
          </div>
          {!clips.length && (
            <p>
              No compatible clips match. The library may still contain other one-shots or loops.
            </p>
          )}
        </>
      )}
    </section>
  );
}
