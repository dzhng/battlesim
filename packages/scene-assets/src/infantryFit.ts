// A soldier's fit to the frame every squad shares: its sockets, standing on
// the ground, its height, eye and muzzle in the standing aim, and each kit's
// muzzle at its declared bore in that kit's active pose. The validator
// (`validate.ts`) runs it on every soldier appearance.

import { vec3 } from "math";
import { pointAt } from "./trs.ts";
import type { MountRow } from "./units.ts";
import { positionsBounds, sampleClip, skinPositions, worldTransforms } from "./pose.ts";
import { groundFindings } from "./boxFit.ts";
import {
  INFANTRY_SOCKETS,
  type Authority,
  type Finding,
  type Joint,
  type MeshData,
  type PoseRef,
  type SkeletonClips,
  type Socket,
  type Tolerances,
} from "./schema.ts";
import { finding, fmt } from "./finding.ts";

export function infantryFindings(
  label: string,
  joints: Joint[],
  mesh: MeshData,
  sockets: Socket[],
  clips: SkeletonClips,
  aim: PoseRef,
  authority: Authority,
  tolerances: Tolerances,
  grounded: readonly MountRow[],
): Finding[] {
  const out: Finding[] = [];
  for (const name of INFANTRY_SOCKETS)
    if (!sockets.some((s) => s.name === name))
      out.push(
        finding(
          "nodes.missing",
          `${label}: no "${name}" socket`,
          `add an empty named "${name}" parented to the right bone (eye: Head; muzzle: the weapon on hand_r)`,
        ),
      );
  const clip = clips.clips.find((c) => c.name === aim.clip);
  const locals = clip ? sampleClip(clips, clip, joints, aim.phase) : joints.map((j) => j.bind);
  const worlds = worldTransforms(
    joints.map((j) => j.parent),
    locals,
  );
  const pose = clip ? `in "${aim.clip}"` : "at bind (no aim reference clip)";
  const bounds = positionsBounds(skinPositions(mesh, joints, worlds));
  out.push(...groundFindings(label, bounds.min[2], tolerances));
  const socketAt = (name: string) => {
    const socket = sockets.find((s) => s.name === name);
    return socket ? pointAt(worlds[socket.joint], socket.offset.t) : null;
  };
  const eye = socketAt("eye");
  const muzzle = socketAt("muzzle");
  const height = bounds.max[2];
  if (Math.abs(height - authority.soldier_height_m) > tolerances.soldier_height_m)
    out.push(
      finding(
        "fit.soldier_height",
        `${label}: stands ${fmt(height)} m ${pose}, physics.soldier_height_m is ${authority.soldier_height_m} ± ${tolerances.soldier_height_m}`,
        "scale the body to the simulation's soldier height",
      ),
    );
  if (eye) {
    if (eye[2] < height * 0.5)
      out.push(
        finding(
          "basis.up",
          `${label}: eye at z ${fmt(eye[2])} m is below half the ${fmt(height)} m height`,
          "export with +Y up in glTF (Blender's default); the bake turns it to Z up",
        ),
      );
    else if (Math.abs(eye[2] - authority.infantry_eye_m) > tolerances.eye_m)
      out.push(
        finding(
          "fit.eye",
          `${label}: eye at ${fmt(eye[2])} m ${pose}, physics.infantry_eye_m is ${authority.infantry_eye_m} ± ${tolerances.eye_m}`,
          "move the eye socket, or rescale the body",
        ),
      );
  }
  if (muzzle && clip) {
    if (muzzle[0] <= 0)
      out.push(
        finding(
          "basis.forward",
          `${label}: muzzle at x ${fmt(muzzle[0])} m ${pose}, behind the origin`,
          "face the model along +X after the catalog's basis_yaw_deg (Quaternius rigs need 90)",
        ),
      );
    else if (
      !grounded.length &&
      Math.abs(muzzle[2] - authority.infantry_muzzle_m) > tolerances.muzzle_m
    )
      out.push(
        finding(
          "fit.muzzle",
          `${label}: muzzle at ${fmt(muzzle[2])} m ${pose}, physics.infantry_muzzle_m is ${authority.infantry_muzzle_m} ± ${tolerances.muzzle_m}`,
          "fix the standing-aim pose or the weapon's muzzle socket",
        ),
      );
  }
  for (const mount of grounded) {
    const hold = mount.operator_appearance!.active_pose!;
    const heldClip = clips.clips.find((c) => c.name === hold.clip);
    if (!heldClip) {
      out.push(
        finding(
          "structure.clips",
          `${label}: active_pose clip "${hold.clip}" is absent`,
          "author the declared supported hold on this skeleton",
        ),
      );
      continue;
    }
    const socket = sockets.find((s) => s.name === "muzzle");
    if (!socket) continue;
    const heldWorlds = worldTransforms(
      joints.map((j) => j.parent),
      sampleClip(clips, heldClip, joints, hold.phase),
    );
    const actual = pointAt(heldWorlds[socket.joint], socket.offset.t);
    const expected = vec3.add([0, 0, 0], mount.pivot_m, mount.muzzle_m!);
    const error = vec3.distance(actual, expected);
    if (error > tolerances.muzzle_m)
      out.push(
        finding(
          "fit.muzzle",
          `${label}: muzzle [${actual.map(fmt).join(", ")}] in active_pose "${hold.clip}" at ${hold.phase}, mount "${mount.id}" declares [${expected.map(fmt).join(", ")}] (${fmt(error)} m off, tolerance ${tolerances.muzzle_m})`,
          "fit the supported active kit's socket to its physical mount bore",
        ),
      );
  }
  return out;
}
