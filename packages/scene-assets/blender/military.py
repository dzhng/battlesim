"""Military bodies (`fixtures/props/military/works.json` and `camp.json`), each authored to
one simulation box: the presets' `street_props.bodies`, or for remains the destroyed
body's plan at the height the catalog leaves. Each kind is a function that returns its
box's half extents, in the module of its family: `military_hangar`, `military_works`
and `military_camp`, on the helpers of `military_parts`. Generic: no nation, unit, brand or sign.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/military.py <kind> <out.glb> [--lint]

`--lint` checks the built model before export and refuses it on a finding: no part more
than 2 cm from every other (floating), the model on the ground, and no two of its bodies
(the root's children: a tent's canvas and its poles, a flag and its pole) cutting into
each other by more than 5 mm (`vehicle_lint`'s checks).

A modular body (HESCO) is authored as one module, which the battle repeats along a placed
panel. Origin at the box's centre on the ground, +X along its first half extent.
"""
import bpy, sys, os, json

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import *
import military_parts
import military_hangar  # noqa: F401 (they register their kinds)

FLAGS = [a for a in script_args() if a.startswith("--")]
ARGS = [a for a in script_args() if not a.startswith("--")]
KIND, OUT = ARGS[0], ARGS[1]
reset()
root = military_parts.ROOT = empty(KIND)
box_half = military_parts.KINDS[KIND]()
rest_on_ground()
bpy.context.view_layer.update()


def lint():
    """The model's findings: floating parts, ground contact and bodies cutting into each
    other (`vehicle_lint`'s checks, each body a child of the root)."""
    import vehicle_lint as VL
    from vehicle_lint_core import LintIssue, LintResult

    def under_root(o):
        chain = [o]
        while chain[-1].parent is not None and chain[-1].parent is not root:
            chain.append(chain[-1].parent)
        return VL._base_part_name(chain[-1].name)

    VL._part_group = under_root
    result = LintResult(vehicle_id=KIND)
    VL.check_floating_parts(result)
    VL.check_interpenetration(result)
    low = min((o.matrix_world @ v.co).z for o in bpy.data.objects if o.type == "MESH" and tier_of(o) in (0, None)
              for v in o.data.vertices)
    if abs(low) > VL.GROUND_CONTACT_TOLERANCE:
        result.add(LintIssue(check="ground", severity="error", message=f"lowest point {low:.3f} m off the ground"))
    for issue in result.issues:
        print("LINT", issue.severity, issue.message)
    if not result.passed:
        raise SystemExit(f"{KIND}: lint found {len(result.issues)} issue(s)")
    print("LINT ok", KIND)


if "--lint" in FLAGS:
    lint()
finish(ao_distance=2.0, ao_strength=0.6, ao_rays=10, paint_scale=1.3)
print("PROP", json.dumps(dict(kind=KIND, box=box_half, tris=triangles_by_tier())))
export(OUT)
