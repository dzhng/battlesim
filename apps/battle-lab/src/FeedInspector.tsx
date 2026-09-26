import type { ObservationView, WeaponPoseView } from "@web/battle/sim/observation";

const deg = (rad: number) => `${((rad * 180) / Math.PI).toFixed(1)}°`;

function Pose({ pose }: { pose: WeaponPoseView }) {
  return (
    <span>
      m{pose.mount} {deg(pose.bearing)} ↑{deg(pose.elevation)} · {pose.shots} shots
    </span>
  );
}

/**
 * The decoded animation feed of one frame: every mount's pose, soldier ids,
 * this tick's tracers and blasts, and the latest fallen. A diagnostic view of
 * what the renderer's pose driver reads, nothing derived from it.
 */
export function FeedInspector({ observation: o }: { observation: ObservationView }) {
  return (
    <div data-testid="feed-panel" className="lab-feed">
      <div className="lab-hint">Animation feed, tick {o.tick}</div>
      {o.own.map((u) => (
        <div key={`own-${u.id}`} data-feed="own">
          own {u.kind} #{u.id}:{" "}
          {u.weaponPoses.map((p) => (
            <Pose key={p.mount} pose={p} />
          ))}
          {u.memberIds.length > 0 && (
            <div className="lab-hint">soldiers {u.memberIds.join(" ")}</div>
          )}
        </div>
      ))}
      {o.identified.map((e) => (
        <div key={`seen-${e.id}`} data-feed="identified">
          enemy {e.kind} {e.id}:{" "}
          {e.weaponPoses.map((p) => (
            <Pose key={p.mount} pose={p} />
          ))}
          {e.memberIds.length > 0 && <div className="lab-hint">seen {e.memberIds.join(" ")}</div>}
        </div>
      ))}
      <div data-feed="tracers">
        tracers:{" "}
        {o.projectiles.length === 0
          ? "none"
          : o.projectiles
              .map(
                (p) =>
                  `${p.own ? "own" : "enemy"} ${p.kind}` +
                  (p.shooterMember === null ? "" : ` by ${p.shooterMember}`) +
                  (p.hit === "none" ? "" : ` → ${p.hit}`),
              )
              .join(" · ")}
      </div>
      <div data-feed="blasts">
        blasts:{" "}
        {o.blasts.length === 0
          ? "none"
          : o.blasts.map((b) => `${b.kind} r${b.radius} m`).join(" · ")}
      </div>
      <div data-feed="corpses">
        fallen:{" "}
        {o.corpses.length === 0
          ? "none"
          : o.corpses
              .slice(-4)
              .map((c) => `${c.own ? "own" : "enemy"} ${c.kind} ${c.soldier} ${deg(c.yaw)}`)
              .join(" · ")}
      </div>
    </div>
  );
}
