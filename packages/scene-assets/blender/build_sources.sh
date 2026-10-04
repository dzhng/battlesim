#!/bin/sh
# Rebuild the standard Blender-scripted appearance batch into assets/source/.
# Run from anywhere; then `asset bake`.
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
v="$root/assets/source/vehicles"
b="$root/assets/source/village"
mkdir -p "$v" "$b"
blender() {
  script=$1; shift
  if output=$(cd "$root/web" && node asset.mjs blender "$here/$script" "$@"); then
    # A successful exporter need not print a summary line.
    printf '%s\n' "$output" | grep -E '^(TANK|TRUCK|JEEP|HOUSE|PROP|wrote) ' || [ "$?" -eq 1 ]
  else
    status=$?
    printf '%s\n' "$output"
    return "$status"
  fi
}
for variant in a b c; do blender infantry_kit.py at_carried "$variant"; done
blender tank.py "$v/tank.glb"
blender tank.py "$v/tank_wreck.glb" --wreck
blender supply_truck.py "$v/supply_truck.glb"
blender supply_truck.py "$v/supply_truck_wreck.glb" --wreck
blender jeep.py "$v/jeep.glb"
blender jeep.py "$v/jeep_wreck.glb" --wreck
# the ruin a loose ruin or rubble prop is drawn as (buildings are a city set: city/village.py)
blender house.py "$b/ruin.glb" 15 12 4 0 --ruin 2
blender props.py wall "$b/wall.glb"
blender props.py crate "$b/crate.glb"
blender props.py bridge_deck "$b/bridge_deck.glb"
for kind in fence sandbags tooth; do blender props.py "$kind" "$b/$kind.glb"; done
# the street's bodies (fixtures/props/city/street.json), one GLB per kind
s="$root/assets/source/street"
mkdir -p "$s"
blender street_car.py "$s/parked_car.glb"
blender street_car.py "$s/car_wreck.glb" --wreck
for kind in jersey_barrier bollard lamp bench bins hydrant utility_box planter bus_shelter \
  heras_fence skip_bin pallet_stack site_cabin traffic_cone road_barrier scaffold scooter; do
  blender street.py "$kind" "$s/$kind.glb"
done
# the gardens' and courts' bodies (fixtures/props/city/gardens.json, courts.json), one GLB
# per kind, and each region's look of a shared piece (`<kind>_<family>`)
c="$root/assets/source/courts"
mkdir -p "$c"
for kind in garden_shed hedge garden_fence washing_line garden_table bike_rack playground_frame swing \
  bike_shed pingpong_table outdoor_gym laundry_poles bench_china bins_china; do
  blender courts.py "$kind" "$c/$kind.glb"
done
# the trees and hedgerows, one GLB per kind into assets/source/trees/
blender trees.py
# the forest floor's bodies and dressing, one GLB per kind into assets/source/forest/
blender forest_floor.py
