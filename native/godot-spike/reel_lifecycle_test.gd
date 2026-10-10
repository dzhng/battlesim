extends SceneTree

const Reel = preload("res://reel.gd")
var failures: Array[String] = []

func _init() -> void:
	call_deferred("run")

func check(condition: bool, message: String) -> void:
	if not condition:
		failures.append(message)
		push_error(message)

func run() -> void:
	var directory := ProjectSettings.globalize_path("res://../../throwaway/reel-lifecycle-test")
	DirAccess.make_dir_recursive_absolute(directory)
	var pose := {"target": [10, 10], "distance": 20, "yaw": 0, "pitch": 0.5}
	var scenes: Array = []
	for id in ["first", "second"]:
		DirAccess.make_dir_recursive_absolute(directory.path_join(id))
		var file := FileAccess.open(directory.path_join(id).path_join("map.json"), FileAccess.WRITE)
		var buildings: Array = [] if id == "second" else [{"template_id": "china-home-10x8-1f", "frame": {"translation": [4, 0, 4], "yaw": 0.0}}]
		file.store_string(JSON.stringify({"size": [20, 20], "surfaces": [{"kind": "paving", "shape": {"kind": "polygon", "ring": [[1, 1], [19, 1], [19, 19], [1, 19]]}}], "forests": [{"shape": {"kind": "polygon", "ring": [[2, 2], [8, 2], [8, 7], [2, 7]]}}], "props": [], "buildings": buildings, "regional_family": "china"}))
		file.close()
		scenes.append({"map": id, "reel": {"shots": [{"seconds": 1, "from": pose, "to": pose}]}})
	var source := directory.path_join("reel.json")
	var file := FileAccess.open(source, FileAccess.WRITE)
	file.store_string(JSON.stringify({"scenes": scenes}))
	file.close()
	for key in ["GODOT_EXIT_AFTER_READY", "GODOT_PRESENTATION_CAPTURE", "GODOT_PRESENTATION_CAPTURE_DIR", "GODOT_AUTHORED_SCENE", "GODOT_AUTHORED_SCENES", "GODOT_REEL_CUTS"]:
		OS.unset_environment(key)
	OS.set_environment("GODOT_REEL_SOURCE", source)
	OS.set_environment("GODOT_AUTHORED_MAP_DIR", directory)
	OS.set_environment("GODOT_AUTHORED_BUILDING_LIMIT", "7")
	OS.set_environment("GODOT_REEL_TIME_SCALE", "1")
	var reel = Reel.new()
	root.add_child(reel)
	reel.set_process(false)
	check(reel._viewport_image() == null, "headless lifecycle runs must not attempt a drawable viewport readback")
	check(not reel._display_backed(), "headless lifecycle runs must disclose that no display backend is active")
	check(reel._resolve_path("throwaway") == ProjectSettings.globalize_path("res://../../throwaway"), "relative renderer paths must resolve from the repository root")
	check(reel._resolve_path("throwaway/report.json") == ProjectSettings.globalize_path("res://../../throwaway/report.json"), "relative report paths must resolve from the repository root")
	check(reel._authored_map_directory() == directory, "authored map override must be the shared map directory")
	check(reel._comparison_blocker().contains("authored model catalog"), "missing authored assets must name the catalog blocker")
	check(not reel._comparison_gates(0).authored_catalog_complete, "comparison gates must keep missing authored assets red")
	check(not reel.proxy_field_used, "map composition must not fall back to the synthetic cube field")
	check(reel.map_geometry_counts["first"].building_limit == 0, "map geometry must default to the complete building export")
	check(reel.map_geometry_counts["first"].prop_limit == 0, "map geometry must default to the complete prop export")
	check(reel.map_geometry_counts["first"].road_limit == 0, "map geometry must default to the complete road export")
	check(reel.map_geometry_counts["first"].forests == 1, "map composition must consume forest regions from the saved export")
	check(reel.map_geometry_counts["first"].rendered_surface_polygons == 1, "map composition must consume polygon surface material")
	check(reel.map_geometry_counts["first"].rendered_buildings == 1, "map report must expose submitted building geometry separately from admitted rows")
	var forest_patch: ArrayMesh = reel._forest_patch_mesh([[2, 2], [8, 2], [8, 7], [2, 7]])
	check(forest_patch != null and forest_patch.get_surface_count() == 1, "forest polygons must produce a triangulated surface instead of a bounding box")
	var grass_centers: Array = reel._grass_centers_for_map({
		"regional_family": "china",
		"forests": [{"shape": {"kind": "polygon", "ring": [[2, 2], [8, 2], [8, 7], [2, 7]]}}],
	}, [Vector2(5, 4.5)])
	check(grass_centers.has(Vector2(5, 4.5)), "catalog grass placement must follow an admitted forest near the captured camera target")
	check(reel.authored_building_limit == 7, "authored building cap must apply before selecting a single or multi-kit scene")
	check(reel._authored_scene_for_template("china-home-10x8-1f", "china") == null, "template catalog lookup must disclose absent authored kits")
	check(reel._authored_family_key("res://authoring/homes-kit.glb") == "homes", "authored kit family must come from the resource path")
	check(reel._authored_family_key("res://authoring/apartments_paris/kit.glb") == "paris_apartments", "Paris apartment kit must map to the regional apartment family")
	check(reel._authored_family_key("res://authoring/china_apartments/kit.glb") == "china_apartments", "China apartment kit must map to the regional apartment family")
	check(reel._authored_shell_token("home-10x8-1f") == "home_10x8_1f_shell", "home template must select its authored shell token")
	check(reel._catalog_prop_source("parked_car", "china") == "assets/source/street/parked_car.glb", "catalog resolver must expose the authored parked-car source")
	check(reel._native_unit_source("menu_rifle") == "assets/source/roster/infantry/rifle_squad/active_a.glb", "catalog resolver must expose the menu infantry appearance source")
	reel.authored_scene_by_family.clear()
	reel.authored_unresolved_templates.clear()
	reel._build_authored_maps(null)
	check(int(reel.authored_unresolved_templates.get("china-home-10x8-1f", 0)) == 1, "missing authored kit must be reported as unresolved")
	check(reel.get_node("MapGeometry_first").visible, "first map must open visible")
	check(not reel.get_node("MapGeometry_second").visible, "second map must start hidden")
	reel._process(1.0)
	check(not reel.get_node("MapGeometry_first").visible, "scene transition must hide first map geometry")
	check(reel.get_node("MapGeometry_second").visible, "scene transition must show second map geometry")
	reel.free()
	OS.unset_environment("GODOT_AUTHORED_MAP_DIR")
	OS.unset_environment("GODOT_AUTHORED_BUILDING_LIMIT")
	var fallback = Reel.new()
	root.add_child(fallback)
	fallback.set_process(false)
	check(fallback._authored_map_directory() == ProjectSettings.globalize_path("res://../../fixtures/maps"), "authored maps must default to the checked-in fixtures directory")
	check(fallback.proxy_field_used, "fallback report must disclose synthetic cube field")
	fallback.free()
	for id in ["first", "second"]:
		DirAccess.remove_absolute(directory.path_join(id).path_join("map.json"))
		DirAccess.remove_absolute(directory.path_join(id))
	DirAccess.remove_absolute(source)
	DirAccess.remove_absolute(directory)
	print(JSON.stringify({"candidate": "reel-lifecycle", "failures": failures}))
	quit(0 if failures.is_empty() else 1)
