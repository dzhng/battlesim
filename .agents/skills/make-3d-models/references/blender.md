# Blender traps

Scripts that run cleanly and produce the wrong thing:

- **glTF imports use quaternion rotation.** Setting `rotation_euler` is
  silently ignored until you set `rotation_mode = "XYZ"`.
- **glTF images arrive packed.** Calling `pack()` again on an unmodified
  packed image drops its data, and the model renders magenta. Repack only
  images you changed.
- **Relative paths resolve against Blender's directory,** not yours. Pass
  absolute paths.
- **Frame from world-space vertices.** `bound_box` is in object space and
  ignores rotation and parents.
- **Rotate the mesh data, not the object,** when normalising a generated
  model: anything that reads vertices never sees an object's turn.
- **Workbench needs a display context.** When one isn't available headless, a
  quick Cycles clay render stands in. Keep to one Cycles job at a time, at a
  small size, while iterating.
- **Use the pinned version.** Exports must repeat byte for byte, and another
  Blender version changes them.
