# Procedural terrain relief

**Status (2026-09-29):** deferred placeholder; no implementation ladder yet.

## Purpose

Add hills and ridges to generated battle maps once the flat-ground generator is established. Relief should create readable defensive positions, masked approaches and changes in sight without eliminating the urban areas and surrounding plains required by the map design.

The initial map generator uses broadly flat ground. Local riverbeds and banks belong to the existing river contract; hills and ridges are this future spec's scope.

## Next Agent Prompt

When the user starts terrain-relief work, read the current procedural map plan in `specs/city-maps/` and run explore-unknowns before writing implementation slices. Establish the desired landforms and their tactical effects, inspect the terrain and navigation owners, and measure the chosen map sizes. This placeholder does not authorize terrain implementation during the city-maps pass.

## Principles to carry forward

- The simulation, navigation, projectile flight, renderer and ground picking consume the same physical terrain. A visual hill must have matching height and sight effects.
- Roads, settlements and building foundations must fit the terrain while preserving accessible approaches and useful plains.
- Relief generation is deterministic from the map's pinned inputs, with bounded work. It should fit the existing map generator's ownership instead of creating a second terrain pipeline.
- Camera clearance must cover terrain as well as buildings; changes in elevation must not put the camera below ground.
- Scale gates cover terrain memory, navigation, sight and rendering at the selected map sizes. Results must be measured before committing to sampling resolution or storage.

## Open before slicing

- Which landforms and height ranges belong in each map type?
- How much flatter ground must surround settlements, and how should roads cross ridges or valleys?
- How should terrain resolution vary with local detail without disagreeing across consumers?
- Which height representation fits the generator's eventual scale architecture?
- Which battle scenarios demonstrate useful sight and movement effects from relief?

These choices remain open for the future walk; this placeholder does not select a noise algorithm, mesh structure or heightmap format.
