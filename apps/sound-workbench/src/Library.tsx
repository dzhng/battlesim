import { useEffect, useRef, useState } from "react";
import { SOUNDS } from "../../../packages/battle-audio/src/synth";
import { RecipeEditor } from "./RecipeEditor";
import type { EditorProps } from "./editorProps";
export type Selection = { kind: "clip" | "sound"; id: string };
export function Library({
  draft,
  edit,
  play,
  search,
  setSearch,
  selected,
  select,
  clone,
}: Omit<EditorProps, "snapshot"> & {
  search: string;
  setSearch(value: string): void;
  selected: Selection | null;
  select(value: Selection): void;
  clone(id: string): void;
}) {
  const [filter, setFilter] = useState("all");
  const activeRow = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    activeRow.current?.scrollIntoView({ block: "nearest" });
  }, [selected, filter]);
  const matches = (text: string) => text.toLowerCase().includes(search.toLowerCase());
  const firing = [
    ...Object.values(draft.defaults),
    ...Object.values(draft.units).flatMap((mounts) => Object.values(mounts)),
  ];
  const assigned = new Set([
    ...firing.flatMap((choice) => [choice.near, choice.far]),
    ...Object.values(draft.impacts).flatMap((rounds) => Object.values(rounds)),
    ...Object.values(draft.effects),
  ]);
  const usedClips = new Set([...assigned].flatMap((name) => draft.sounds[name].clips));
  return (
    <div className="sw-layout">
      <aside className="sw-browser">
        <label>
          Search library
          <input
            type="search"
            aria-label="Search library"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
        <label>
          Show
          <select
            aria-label="Library filter"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          >
            <option value="all">All clips and recipes</option>
            <option value="clips">Recorded clips</option>
            <option value="sounds">Recipes and baselines</option>
            <option value="baselines">Original synthesis baselines</option>
            <option value="unused">Unused recorded clips</option>
            <option value="reload">Reload and mechanical</option>
          </select>
        </label>
        <p className="sw-count">
          {Object.keys(draft.clips).length} recorded clips · {Object.keys(draft.sounds).length}{" "}
          recipes
        </p>
        {Object.entries(draft.clips)
          .filter(
            ([id, clip]) =>
              !["sounds", "baselines"].includes(filter) &&
              (filter !== "reload" || ["reload", "mechanical"].includes(clip.role)) &&
              (filter !== "unused" || !usedClips.has(id)) &&
              matches(`${id} ${clip.label} ${clip.category} ${clip.role}`),
          )
          .map(([id, clip]) => (
            <button
              className="sw-library-row"
              aria-label={clip.label}
              aria-pressed={selected?.kind === "clip" && selected.id === id}
              key={`clip/${id}`}
              ref={selected?.kind === "clip" && selected.id === id ? activeRow : undefined}
              onClick={() => select({ kind: "clip", id })}
            >
              <span>{clip.label}</span>
              <small>
                {clip.role} · {(clip.frames / clip.sample_rate).toFixed(2)} s
              </small>
            </button>
          ))}
        {Object.entries(draft.sounds)
          .filter(
            ([id, sound]) =>
              ["all", "sounds", "baselines"].includes(filter) &&
              (filter !== "baselines" || Object.hasOwn(SOUNDS, id)) &&
              matches(`${id} ${sound.label}`),
          )
          .map(([id, sound]) => (
            <button
              className="sw-library-row"
              aria-label={sound.label}
              aria-pressed={selected?.kind === "sound" && selected.id === id}
              key={`sound/${id}`}
              ref={selected?.kind === "sound" && selected.id === id ? activeRow : undefined}
              onClick={() => select({ kind: "sound", id })}
            >
              <span>{sound.label}</span>
              <small>
                {Object.hasOwn(SOUNDS, id) ? "baseline" : "recipe"} ·{" "}
                {sound.loop
                  ? "loop"
                  : `${Math.max(1, sound.clips.length)} variation${sound.clips.length > 1 ? "s" : ""}`}
              </small>
            </button>
          ))}
      </aside>
      <div className="sw-inspector">
        {selected?.kind === "clip" && draft.clips[selected.id] ? (
          (() => {
            const clip = draft.clips[selected.id];
            const source = draft.sources[clip.source];
            return (
              <section className="sw-detail">
                <p className="sw-eyebrow">CLEAN RECORDING · {clip.role.toUpperCase()}</p>
                <h2>{clip.label}</h2>
                <p className="sw-id">{selected.id}</p>
                <button onClick={() => void play("clip", selected.id)}>Play clip</button>
                <p>{clip.notes}</p>
                <dl>
                  <dt>Family</dt>
                  <dd>{clip.category}</dd>
                  <dt>Duration</dt>
                  <dd>
                    {(clip.frames / clip.sample_rate).toFixed(3)} s · {clip.sample_rate} Hz
                  </dd>
                  <dt>Source</dt>
                  <dd>
                    <a href={source.url} target="_blank" rel="noreferrer">
                      {source.label}
                    </a>{" "}
                    · {source.author} · {source.license}
                  </dd>
                  <dt>Source crop</dt>
                  <dd>
                    {(clip.source_frames[0] / clip.source_rate).toFixed(3)}–
                    {(clip.source_frames[1] / clip.source_rate).toFixed(3)} s
                  </dd>
                  <dt>Processing</dt>
                  <dd>{clip.processing}</dd>
                </dl>
                {source.notes && <p>{source.notes}</p>}
                {["reload", "mechanical"].includes(clip.role) && (
                  <p>
                    Stored for audition. The current battle feed has no reload or mechanical-action
                    event.
                  </p>
                )}
              </section>
            );
          })()
        ) : selected?.kind === "sound" && draft.sounds[selected.id] ? (
          <RecipeEditor
            key={selected.id}
            id={selected.id}
            catalog={draft}
            change={(recipe) =>
              edit((c) => {
                c.sounds[selected.id] = recipe;
              })
            }
            clone={() => {
              setFilter("sounds");
              clone(selected.id);
            }}
            play={(variation) => void play("sound", selected.id, variation)}
          />
        ) : (
          <section className="sw-detail sw-empty">
            <h2>Choose a clip or recipe</h2>
            <p>
              The complete library includes unused reloads, mechanical actions and every original
              synthesis baseline.
            </p>
          </section>
        )}
      </div>
    </div>
  );
}
