extends RefCounted

## The scene assets' appearance catalog (`assets/catalog.json`), as the
## browser's one loader names what it draws: an appearance and one of its
## states. A kit's states are its modules, root nodes of the kit's one GLB; any
## other appearance's state names a GLB of its own, whose root node is the
## model. Native resolves names here and nowhere else, stages the files, and
## hands each layer the node tree of one state.

const Staging = preload("res://assets/staging.gd")

const PATH := "assets/catalog.json"

var _entries: Dictionary
## Repository path -> staged resource path.
var _staged := {}
## Repository path -> its instantiated scene, kept until `free_scenes`.
var _scenes := {}

func _init() -> void:
	var catalog = JSON.parse_string(FileAccess.get_file_as_string(Staging.repo_path(PATH)))
	_entries = catalog.appearances if typeof(catalog) == TYPE_DICTIONARY and typeof(catalog.get("appearances")) == TYPE_DICTIONARY else {}

func has(appearance: String) -> bool:
	return _entries.has(appearance)

## The repository GLB that draws `state` of `appearance`, or "" for none.
func source(appearance: String, state: String) -> String:
	var entry = _entries.get(appearance)
	if typeof(entry) != TYPE_DICTIONARY:
		return ""
	if entry.get("unit") == "kit":
		return String(entry.get("source", ""))
	var states = entry.get("states", {})
	return String(states.get(state, "")) if typeof(states) == TYPE_DICTIONARY else ""

## Stages `sources` (repository paths), blockers into `report`.
func stage(sources: Array, report) -> void:
	_staged.merge(Staging.stage(sources, report), true)

## The node tree drawing `state` of `appearance` (its meshes below it), or
## null when it is not staged or has no such module. Owned by the catalog.
func node(appearance: String, state: String) -> Node:
	var from := source(appearance, state)
	if not _staged.has(from):
		return null
	if not _scenes.has(from):
		_scenes[from] = (load(_staged[from]) as PackedScene).instantiate()
	var scene: Node = _scenes[from]
	if _entries[appearance].get("unit") == "kit":
		return scene.get_node_or_null(NodePath(state))
	return scene

## Frees every scene `node` instantiated: call once the layers have built.
func free_scenes() -> void:
	for scene in _scenes.values():
		scene.free()
	_scenes.clear()
