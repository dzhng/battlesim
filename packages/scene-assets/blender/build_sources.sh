#!/bin/sh
# Rebuild every Blender-scripted appearance source into assets/source/.
# Run from anywhere; then `asset bake`.
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
v="$root/assets/source/vehicles"
b="$root/assets/source/village"
mkdir -p "$v" "$b"
blender() { script=$1; shift; (cd "$root/web" && node asset.mjs blender "$here/$script" "$@") | grep -E '^(TANK|TRUCK|JEEP|HOUSE|PROP|wrote) ' ; }
blender tank.py "$v/tank.glb"
blender tank.py "$v/tank_wreck.glb" --wreck
blender supply_truck.py "$v/supply_truck.glb"
blender supply_truck.py "$v/supply_truck_wreck.glb" --wreck
blender jeep.py "$v/jeep.glb"
blender jeep.py "$v/jeep_wreck.glb" --wreck
# the village map's three buildings (fixtures/game.json map.buildings), each intact and ruined
blender house.py "$b/house_a.glb" 15 12 4 0
blender house.py "$b/house_a_ruin.glb" 15 12 4 0 --ruin 2
blender house.py "$b/house_b.glb" 17 14 4 1
blender house.py "$b/house_b_ruin.glb" 17 14 4 1 --ruin 2
blender house.py "$b/house_c.glb" 13 11 4 2
blender house.py "$b/house_c_ruin.glb" 13 11 4 2 --ruin 2
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
# the trees and hedgerows, one GLB per kind into assets/source/trees/
blender trees.py
# the forest floor's bodies and dressing, one GLB per kind into assets/source/forest/
blender forest_floor.py
