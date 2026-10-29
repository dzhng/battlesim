// Assignment authoring belongs to the look document; the library edits recipes.
import type { EditorProps } from "./editorProps";
export function GlobalAssignments({ snapshot, play }: EditorProps) {
  return (
    <section className="sw-detail sw-global">
      <h2>Weapon looks</h2>
      {Object.entries(snapshot.looks).map(([name, look]) => (
        <div className="sw-assignment" key={name}>
          <h3>{name}</h3>
          <button onClick={() => void play("sound", look.sound.shot.near, 0, look.sound.shot.gain)}>
            Play near
          </button>
          <button onClick={() => void play("sound", look.sound.shot.far, 0, look.sound.shot.gain)}>
            Play far
          </button>
        </div>
      ))}
    </section>
  );
}
