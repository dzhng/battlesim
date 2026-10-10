extends "res://tests/check.gd"

const Manifest = preload("res://capture/visual_manifest.gd")

func run() -> bool:
	var fixture_dir := FIXTURE_RUN.path_join(FIXTURE_SCENE)
	var loaded := Manifest.load(fixture_dir, FIXTURE_MAPS)
	check(loaded.ok, "the fixture manifest is admitted: %s" % loaded.get("blocker"))
	check(loaded.get("manifest", {}).get("layers", []).has("buildings") and not loaded.manifest.layers.has("fog"), "the admitted manifest carries the layers the browser drew")
	check(loaded.get("capture", {}).get("valid", false), "the admitted run carries its parsed capture")

	# Each identity mismatch is refused with a blocker that names it.
	var scratch_root := scratch("visual-manifest")
	var maps := scratch_root.path_join("maps")
	_copy_fixture(scratch_root, maps)
	var run := scratch_root.path_join(FIXTURE_SCENE)
	check(Manifest.load(run, maps).ok, "an unchanged copy is admitted")

	_append(maps.path_join(FIXTURE_SCENE).path_join("map.json"), " ")
	var map_changed := Manifest.load(run, maps)
	check(not map_changed.ok and map_changed.blocker.name == "identity" and map_changed.blocker.detail.contains("mapHash"), "a changed map is refused by its hash")

	_copy_fixture(scratch_root, maps)
	_append(run.path_join("capture.json"), " ")
	var capture_changed := Manifest.load(run, maps)
	check(not capture_changed.ok and capture_changed.blocker.detail.contains("captureHash"), "a changed capture is refused by its hash")

	_copy_fixture(scratch_root, maps)
	var manifest: Dictionary = read_json(run.path_join("manifest.json"))
	manifest.identity.seed = 2
	_write(run.path_join("manifest.json"), JSON.stringify(manifest))
	var seed_changed := Manifest.load(run, maps)
	check(not seed_changed.ok and seed_changed.blocker.detail.contains("seed"), "a manifest naming another seed than its capture is refused")

	_copy_fixture(scratch_root, maps)
	manifest = read_json(run.path_join("manifest.json"))
	manifest.lens.far = 4000
	_write(run.path_join("manifest.json"), JSON.stringify(manifest))
	var stale := Manifest.load(run, maps)
	check(not stale.ok and stale.blocker.name == "manifest", "a manifest with a finite far plane is not this format")

	_copy_fixture(scratch_root, maps)
	manifest = read_json(run.path_join("manifest.json"))
	manifest.erase("light")
	_write(run.path_join("manifest.json"), JSON.stringify(manifest))
	check(not Manifest.load(run, maps).ok, "a manifest without the light it was drawn under is not this format")

	for broken in [func(m): m.erase("terrain"), func(m): m.terrain[0].erase("sha256"), func(m): m.erase("tuning"), func(m): m.tuning.erase("grass"), func(m): m.grass.tables[0].erase("layout"), func(m): m.grass.kinds = [], func(m): m.grassCounts = [{"plate": "plate-1.png"}], func(m): m.grassCounts[0].cells = [-1]]:
		_copy_fixture(scratch_root, maps)
		manifest = read_json(run.path_join("manifest.json"))
		broken.call(manifest)
		_write(run.path_join("manifest.json"), JSON.stringify(manifest))
		check(not Manifest.load(run, maps).ok, "a manifest with malformed tuning, terrain, grass or grass counts is refused")

	_copy_fixture(scratch_root, maps)
	DirAccess.remove_absolute(maps.path_join(FIXTURE_SCENE).path_join("map.json"))
	var missing := Manifest.load(run, maps)
	check(not missing.ok and missing.blocker.detail.contains("map.json"), "a missing map is a named blocker")
	return true

func _copy_fixture(root: String, maps: String) -> void:
	var run := root.path_join(FIXTURE_SCENE)
	DirAccess.make_dir_recursive_absolute(run)
	DirAccess.make_dir_recursive_absolute(maps.path_join(FIXTURE_SCENE))
	for file in ["capture.json", "manifest.json"]:
		DirAccess.copy_absolute(ProjectSettings.globalize_path(FIXTURE_RUN.path_join(FIXTURE_SCENE).path_join(file)), run.path_join(file))
	DirAccess.copy_absolute(ProjectSettings.globalize_path(FIXTURE_MAPS.path_join(FIXTURE_SCENE).path_join("map.json")), maps.path_join(FIXTURE_SCENE).path_join("map.json"))

func _append(path: String, text: String) -> void:
	_write(path, FileAccess.get_file_as_string(path) + text)

func _write(path: String, text: String) -> void:
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(text)
	file.close()
