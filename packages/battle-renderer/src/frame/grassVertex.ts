// The grass's vertex stage: each blade of a clump placed, scaled and turned,
// bent in the one wind by the square of its height fraction, and laid over
// where tracks or trampling flattened it.
import { tgpu, d } from "typegpu";
import game from "@fixtures/game.json";
import { typegpuCameraLayout } from "../world/camera";
import { cameraWind } from "../world/camera";
import { grassDrawLayout } from "./grassLayouts";

const tuning = game.presentation.grass;

const GrassVertex = d
  .struct({
    clip: d.vec4f,
    world: d.vec3f,
    root: d.vec3f,
    normal: d.vec3f,
    albedo: d.vec3f,
    /** How far the fragment is lit as the ground under it, not as a blade. */
    ground: d.f32,
  })
  .$name("GrassVertex");

/** One blade vertex of clump `iid` of the bound tier: placed, scaled, turned,
 *  and bent in the wind by the square of its height fraction. */
const grassVertexOf = tgpu
  .fn(
    [d.u32, d.u32],
    GrassVertex,
  )(/* wgsl */ `(vid: u32, iid: u32) -> GrassVertex {
  let P = &grassDrawLayout.$.params;
  let tier = grassDrawLayout.$.tier;
  let c = grassDrawLayout.$.clumps[tier.y + iid];
  let row = (*P).kinds[c.kind];
  var out: GrassVertex;
  if (vid / tier.z >= u32(row.y)) {
    // A blade this kind does not have: outside the depth range, clipped.
    out.clip = vec4f(0.0, 0.0, 2.0, 1.0);
    return out;
  }
  let base = select((*P).bases[c.kind].y, (*P).bases[c.kind].x, tier.x == 0u);
  let s = grassDrawLayout.$.shapes[base + vid];
  let scale = c.height / max(row.x, 1e-4);
  let sn = c.yawSin;
  let cs = c.yawCos;
  let sp = s.spine.xyz * scale;
  let sd = s.side.xyz * scale * c.width;
  let spine = vec3f(sp.x * cs - sp.y * sn, sp.x * sn + sp.y * cs, sp.z);
  let side = vec3f(sd.x * cs - sd.y * sn, sd.x * sn + sd.y * cs, sd.z);
  // The one wind: a steady lean, gust fronts rolling downwind, a flutter.
  let t = typegpuCameraLayout.$.cam.time;
  let wind = cameraWind();
  let dir = wind.toward.xy;
  let along = dot(c.root.xy, dir);
  let front = sin(6.2831853 * (along - t * wind.gustMps) * wind.gustPerM);
  let gust = smoothstep(${tuning.sway.gust_from}, 1.0, front);
  let flutter = (*P).sway.z * sin((*P).sway.w * t + s.side.w * 6.2831853 + along * ${tuning.sway.flutter_phase});
  let bend = s.spine.w * s.spine.w;
  let packed = unpack4x8unorm(c.colour);
  // Laid over by tracks or trampling: the clump sinks and its blades lean
  // out its own way, still in the wind but less. A stiff kind answers the
  // wind less (its shape's own response).
  let flat = 1.0 - packed.w;
  let answer = s.tint.w * (1.0 - flat);
  let lean = ((*P).sway.x + (*P).sway.y * gust + flutter) * answer;
  let push = dir * lean + vec2f(-dir.y, dir.x) * flutter * ${tuning.sway.flutter_across} * answer;
  let laid = vec2f(cs, sn) * flat * ${tuning.laid.lean};
  var world = c.root + vec3f(spine.xy, spine.z * (1.0 - flat * ${tuning.laid.sink})) + side + vec3f((push + laid) * c.height * bend, 0.0);
  world.z -= 0.5 * dot(push, push) * c.height * bend;
  let n = s.normal.xyz;
  let colour = packed.xyz;
  out.clip = typegpuCameraLayout.$.cam.viewProj * vec4f(world, 1.0);
  // Blades cross each other: where two meet at (nearly) one depth the winner
  // is left to the hardware and changes frame to frame. A per-blade nudge of
  // up to a two-thousandth of the depth (5 cm at 100 m, a few millimetres
  // near the eye) separates every such pair the same way each frame.
  let tie = ((bitcast<u32>(c.root.x) * 2654435761u) ^ (bitcast<u32>(c.root.y) * 2246822519u)) + (vid / tier.z) * 374761393u;
  out.clip.z *= 1.0 + 1e-3 * (f32(tie >> 8u) / 16777216.0 - 0.5);
  out.world = world;
  out.root = c.root;
  out.normal = vec3f(n.x * cs - n.y * sn, n.x * sn + n.y * cs, n.z);
  // A clump a few pixels tall shows its tops, not its roots: drawn with them,
  // its dark roots and gaps read as speckle, so its tint and its lighting
  // give way to the ground's.
  let footprint = (*P).eye.w * distance(typegpuCameraLayout.$.cam.eye, c.root);
  let plain = (*P).shading.x * smoothstep((*P).tiers.z, (*P).tiers.w, footprint);
  out.albedo = colour * colour * mix(s.tint.xyz, vec3f(1.0), plain);
  out.ground = mix(1.0 - (*P).shading.y, 1.0, plain);
  return out;
}`)
  .$uses({ grassDrawLayout, typegpuCameraLayout, cameraWind, GrassVertex });

export const grassVertex = tgpu.vertexFn({
  in: { vid: d.builtin.vertexIndex, iid: d.builtin.instanceIndex },
  out: {
    clip: d.builtin.position,
    world: d.vec3f,
    root: d.vec3f,
    normal: d.vec3f,
    albedo: d.vec3f,
    ground: d.f32,
  },
})((v) => {
  "use gpu";
  const g = grassVertexOf(v.vid, v.iid);
  return {
    clip: g.clip,
    world: g.world,
    root: g.root,
    normal: g.normal,
    albedo: g.albedo,
    ground: g.ground,
  };
});
