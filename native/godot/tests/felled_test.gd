extends "res://tests/check.gd"

## A felled tree's fall (`world/felled.gd`) is the browser's
## (`scenery/felled.ts`): the pose a tree 12 m tall takes as it falls, strikes
## the ground and lies, equal to the browser's `fellPose` for the same inputs
## (vectors computed from the TypeScript). Where its points go is the Rust
## extension's, tested against the same vectors (`native/godot-binding`).

const Felled = preload("res://world/felled.gd")

const RULES := {"fall_s": 2.6, "settle_s": 0.6, "settle_deg": 4, "rest_height_m": 1, "rest_spread": 0.6, "stump_height_m": 0.4, "cut": [0.26, 0.19, 0.11]}
## Per age: the pose's angle, squash, spread and landed share, and the four
## points (`felled_vec.ts`, the browser's output).
const BROWSER := {
	1.0: [0.2361924969804743, 1.0, 1.0, 0.0, [Vector3(0.09633515006749871, 0.1951135334233316, 0.3485194411788784), Vector3(0.5643776261984075, 0.38583683493121, 2.9184534603317402), Vector3(2.5453956514184672, 1.893860868557957, 7.484790186609964), Vector3(-0.5050804814757548, 2.993226024698994, 10.799302460852692)]],
	2.8: [1.5563546270494681, 0.26980407583318466, 0.602955966234294, 0.9926100844142647, [Vector3(0.3272458684922279, 0.5029944913229707, 0.08076193305591006), Vector3(2.0935235430631294, 2.424698057417506, 0.2982902768917322), Vector3(5.362256545651307, 6.245241444850302, 0.059783371436827604), Vector3(5.652008024524516, 9.7468492422251, 0.5617260640320374)]],
	5.0: [1.5966612795880064, 0.26436781609195403, 0.6, 1.0, [Vector3(0.3245862068965517, 0.4994482758620689, 0.08007358543362952), Vector3(2.094857522724545, 2.4264766969660605, 0.19277199985940593), Vector3(5.330302360855302, 6.207069814473736, -0.24011493184610655), Vector3(5.6680796085613405, 9.757439478081789, 0.13157382518341704)]],
}

func run() -> bool:
	for age in BROWSER:
		var want: Array = BROWSER[age]
		var pose := Felled.fell_pose(RULES, Vector2(0.6, 0.8), age, 12.0, 3.0, 0.3, 4.0)
		check(is_equal_approx(pose[2], want[0]) and is_equal_approx(pose[5], want[1]) and is_equal_approx(pose[6], want[2]) and is_equal_approx(pose[7], want[3]), "at %s s the pose is the browser's: %s" % [age, pose])
	return true
