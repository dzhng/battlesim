// A skeleton's own checks: its clips build, and it carries every infantry role
// and its aim reference pose.

import { buildClips } from "./build.ts";
import { finding } from "./finding.ts";
import { importScene } from "./scene.ts";
import {
  INFANTRY_CLIPS,
  type Finding,
  type PoseRef,
  type SkeletonClips,
  type SkeletonEntry,
} from "./schema.ts";
import { hasErrors, type Validation } from "./validate.ts";

export async function validateSkeleton(
  id: string,
  entry: SkeletonEntry,
  bytes: Uint8Array,
): Promise<Validation<SkeletonClips>> {
  const { scene, findings } = importScene(bytes, entry.source, entry.basis_yaw_deg);
  if (!scene) return { findings, stats: null, bundle: null, preview: null };
  const built = buildClips(scene, entry.source, id, entry.sample_hz, entry.clips);
  findings.push(...built.findings);
  if (!built.built) return { findings, stats: null, bundle: null, preview: null };
  findings.push(...clipRoleFindings(built.built, entry.aim_reference, entry.source));
  const clips = built.built;
  return {
    findings,
    stats: {
      kind: "clips",
      tiers: [],
      joints: clips.joints.length,
      clips: clips.clips.map((c) => ({
        name: c.name,
        loop: c.loop,
        duration: c.duration,
        frames: c.frames,
      })),
      source_bytes: bytes.byteLength,
    },
    bundle: hasErrors(findings) ? null : clips,
    preview: clips,
  };
}

function clipRoleFindings(clips: SkeletonClips, aim: PoseRef, label: string): Finding[] {
  const names = new Set(clips.clips.map((c) => c.name));
  return [...INFANTRY_CLIPS, aim.clip]
    .filter((name, i, all) => all.indexOf(name) === i && !names.has(name))
    .map((name) =>
      finding(
        "structure.clips",
        `${label}: skeleton ${clips.id} lacks clip "${name}"`,
        name === aim.clip
          ? "author the standing-aim reference pose infantry fit is measured in"
          : `author the "${name}" role on the skeleton`,
      ),
    );
}
