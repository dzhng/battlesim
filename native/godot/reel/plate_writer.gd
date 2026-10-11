extends RefCounted

## Writes the plate for the frame just drawn (`RenderingServer.force_draw`):
## it reads the viewport back and saves a PNG. The plate must be exactly the
## record's canvas; any other size is a blocker, never a resize.

## Returns `{ok: true, gpu_ms}` or `{ok: false, blocker: {name, detail}}`.
## `gpu_ms` is Godot's measured viewport render time (the caller turns the
## measurement on), or null when the driver measured none: Godot 4.7's Metal
## driver records every GPU timestamp as 0.
static func write(viewport: Viewport, record: Dictionary, path: String) -> Dictionary:
	var rid := viewport.get_viewport_rid()
	var image := viewport.get_texture().get_image()
	var canvas := Vector2i(int(record.canvas[0]), int(record.canvas[1]))
	if image.get_size() != canvas:
		return {"ok": false, "blocker": {"name": "plate", "detail": "frame %d drew %s but its canvas is %s" % [record.index, image.get_size(), canvas]}}
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var saved := image.save_png(path)
	if saved != OK:
		return {"ok": false, "blocker": {"name": "plate", "detail": "could not save %s: %s" % [path, error_string(saved)]}}
	var gpu_ms := RenderingServer.viewport_get_measured_render_time_gpu(rid)
	return {"ok": true, "gpu_ms": gpu_ms if gpu_ms > 0.0 else null}
