# Unprimed critique: stand-in screenshots

Looked only at the four images. Coordinates are pixels in the 1280x800 frame.

## What each image shows

- **vehicle-before**: two plain untextured boxes on grass beside a wheat field. Centre box is tan on top with an olive side and trails dust. Foreground box (lower left, cropped by the frame and HUD) is white on top with a teal side.
- **vehicle-after**: the centre box is now a camouflaged 8-wheeled armoured vehicle with a small turret and two antennas. The foreground box is now a flat grey, uncamouflaged turreted hull.
- **squad-before**: two small upright boxes in a wheat field inside a huge red-striped circle with a pale ring, partly under a hatched grey-blue zone. A large pale box sits inside a smoke cloud.
- **squad-after**: the small boxes are two running soldiers; the pale box is a featureless white vehicle silhouette.

## Problems, most serious first

1. **Boxes read as a bug** (both befores, high confidence). Untextured cuboids with nothing marking them as units. A player would think models failed to load. Placeholder.
2. **Box colours differ with no explanation** (vehicle-before, high). Tan/olive versus white/teal; no way to tell if that means side, type or state.
3. **Foreground vehicle is flat grey** (vehicle-after, lower left, medium-high). Next to the camouflaged one it looks untextured or like a wreck; a player cannot tell which.
4. **White vehicle in smoke** (squad-after, 810-1060 x 140-240, high). Blank white shape, plus a small detached white fragment floating below it at about (965, 250). Looks like a missing texture or bug.
5. **Ground overlay draws over the smoke** (both squad images, medium). Red stripes and hatching cross the cloud and the vehicle, so depth reads wrong.
6. **Command bar changes between before and after** (both pairs, high). Before: no button outlines, the right-hand five icons lit. After: outlines appear, four of those are dimmed and one is boxed. Nothing visible explains the change.
7. **No unit markers, selection or side indication anywhere** (all four, high).
8. **Soldiers are hard to see** (squad-after, medium). Dark figures on the dark hatched zone; one stands exactly on the zone edge.
9. **Small boxes have a lighter orange band at the base** (squad-before, medium). Looks sunk into or tinted by the ground overlay.
10. **Detached post left of the camouflaged vehicle's nose** at about (468, 355) (vehicle-after, low-medium). The antennas are also taller than the hull.
11. **The two vehicles nearly touch** (both vehicle images, around (540, 460), medium). Possible clipping.
12. **FPS counter** bottom left is barely legible debug text (all four).

## Before against after

Position, heading and shadows match in both pairs, and the dust plume is identical. Each box is slightly larger than what replaces it: the soldier boxes are clearly wider and a little taller, and the vehicle boxes are a little taller. The swap is a visible shrink, not a jump. I could not confirm anything floating or sinking apart from the white fragment (item 4) and the box bases (item 9).
