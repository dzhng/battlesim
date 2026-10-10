extends MultiMeshInstance3D

const Coords = preload("res://core/coords.gd")

## The fog overlay, drawn only when the manifest lists the `fog` layer: one dark
## translucent tile over every cell the observed side cannot see, from the fog
## bits of the publication being shown. Every cell is drawn; there is no cap.

func _init() -> void:
	name = "Fog"
	multimesh = MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	var tile := BoxMesh.new()
	var material := StandardMaterial3D.new()
	material.albedo_color = Color(0.015, 0.03, 0.06, 0.62)
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	tile.material = material
	multimesh.mesh = tile

## Shows the hidden cells of one decoded fog grid `{nx, ny, cellM, bits}`.
func show_fog(fog: Dictionary) -> void:
	var cell := float(fog.cellM)
	var hidden: Array[Vector2i] = []
	for y in int(fog.ny):
		for x in int(fog.nx):
			var index := y * int(fog.nx) + x
			if index / 32 >= fog.bits.size() or (int(fog.bits[index / 32]) & (1 << (index % 32))) == 0:
				hidden.append(Vector2i(x, y))
	multimesh.instance_count = hidden.size()
	var basis := Basis.from_scale(Vector3(cell, 0.03, cell))
	for i in hidden.size():
		multimesh.set_instance_transform(i, Transform3D(basis, Coords.sim_to_godot(Vector3((hidden[i].x + 0.5) * cell, (hidden[i].y + 0.5) * cell, 0.02))))
