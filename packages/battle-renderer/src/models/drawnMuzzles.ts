// Where each posed model's muzzles are drawn: a vehicle mount's muzzle node
// under its articulation (turret yaw, gun pitch, recoil; the HMG on its own
// mount), and a soldier's weapon `muzzle` socket in his clip. Read from the
// same pose frame the models are drawn from, so a muzzle flash put here sits
// on the drawn barrel however the simulation's own muzzle model differs.
//
// Computed on demand, only for the muzzles asked after (a frame's live
// flashes), and remembered for the rest of the frame.
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import {
  articulate,
  articulationRig,
  restLocals,
  type ArticulationRig,
} from "@packages/scene-assets/src/articulation";
import { poseWorlds, worldTransforms } from "@packages/scene-assets/src/pose";
import { mul, trsMatrix, type Trs } from "@packages/scene-assets/src/trs";
import type { ArticulatedBundle, Side } from "@packages/scene-assets/src/schema";
import { vec3, type Vec3 } from "math";
import type { ResolveAppearance } from "./modelInstances";
import type { MountRole, PoseFrame, SoldierPose, UnitKindName, VehiclePose } from "./poseDriver";
import { sideKey } from "../sideKey";

/** A mount role's muzzle node in an articulated bundle. */
const MUZZLE_NODE: Record<MountRole, string | null> = {
  gun: "muzzle",
  hmg: "hmg_muzzle",
  hand: null,
};

/** An articulated bundle's rig and its node locals to pose into. */
interface Rig {
  rig: ArticulationRig;
  locals: Trs[];
  parents: number[];
}

/** One key per (side, id): blue and red ids never collide. */
const keyOf = (side: Side, id: number) => sideKey(id, side, "blue");

export class DrawnMuzzles {
  private readonly vehicles = new Map<number, VehiclePose>();
  private readonly soldiers = new Map<number, SoldierPose>();
  private readonly rigs = new Map<ArticulatedBundle, Rig>();
  /** This frame's answers: muzzle point (or null, nothing drawn) by query. */
  private readonly memo = new Map<string, Vec3 | null>();

  constructor(
    private readonly installed: InstalledAppearances,
    private readonly resolve: ResolveAppearance,
    /** Mount roles per unit kind, in the rules' mount order (`mountRoles`). */
    private readonly roles: Partial<Record<UnitKindName, MountRole[]>>,
  ) {}

  /** Take the pose frame the models are drawn from this frame. */
  update(frame: PoseFrame) {
    this.vehicles.clear();
    this.soldiers.clear();
    this.memo.clear();
    for (const v of frame.vehicles) this.vehicles.set(keyOf(v.side, v.unit), v);
    for (const s of frame.soldiers) this.soldiers.set(keyOf(s.side, s.soldier), s);
  }

  /** The drawn muzzle of `side`'s vehicle `unit`'s mount `mount`, into `at`;
   *  false when it isn't drawn or its mount has no muzzle node. */
  vehicle(side: Side, unit: number, mount: number, at: Vec3): boolean {
    return this.answer(`v${keyOf(side, unit)}:${mount}`, at, () => {
      const v = this.vehicles.get(keyOf(side, unit));
      const role = v && this.roles[v.kind]?.[mount];
      const name = role ? MUZZLE_NODE[role] : null;
      const bundle = v && name ? this.bundle(v.kind, v.side, v.unit) : null;
      if (!v || !name || bundle?.kind !== "articulated") return null;
      const node = bundle.nodes.findIndex((n) => n.name === name);
      if (node < 0) return null;
      const r = this.rig(bundle);
      articulate(r.locals, bundle.nodes, r.rig, v.articulation);
      const m = worldTransforms(r.parents, r.locals)[node];
      return placed(v.position, v.yaw, m[12], m[13], m[14]);
    });
  }

  /** The drawn muzzle of `side`'s soldier `soldier`'s weapon, into `at`;
   *  false when he isn't drawn or his body has no `muzzle` socket. */
  soldier(side: Side, soldier: number, at: Vec3): boolean {
    return this.answer(`s${keyOf(side, soldier)}`, at, () => {
      const s = this.soldiers.get(keyOf(side, soldier));
      const bundle = s ? this.bundle(s.kind, s.side, s.soldier) : null;
      if (!s || bundle?.kind !== "skinned") return null;
      const socket = bundle.sockets.find((k) => k.name === "muzzle");
      if (!socket) return null;
      const skeleton = this.installed.skeletons.get(bundle.skeleton) ?? null;
      const joint = poseWorlds(bundle, skeleton, s)[socket.joint];
      const m = mul(joint, trsMatrix(socket.offset));
      return placed(s.position, s.facing, m[12], m[13], m[14]);
    });
  }

  private answer(key: string, at: Vec3, find: () => Vec3 | null): boolean {
    let p = this.memo.get(key);
    if (p === undefined) this.memo.set(key, (p = find()));
    if (!p) return false;
    vec3.copy(at, p);
    return true;
  }

  private bundle(kind: UnitKindName, side: Side, id: number) {
    const resolved = this.resolve(kind, side, id);
    return (resolved && this.installed.appearances.get(resolved.appearance)?.bundle) ?? null;
  }

  private rig(bundle: ArticulatedBundle): Rig {
    let r = this.rigs.get(bundle);
    if (!r) {
      r = {
        rig: articulationRig(bundle.nodes),
        locals: restLocals(bundle.nodes),
        parents: bundle.nodes.map((n) => n.parent),
      };
      this.rigs.set(bundle, r);
    }
    return r;
  }
}

/** A model-space point placed at `position`, turned by `yaw` about +Z. */
function placed(position: Vec3, yaw: number, x: number, y: number, z: number): Vec3 {
  const c = Math.cos(yaw);
  const s = Math.sin(yaw);
  return [position[0] + x * c - y * s, position[1] + x * s + y * c, position[2] + z];
}
