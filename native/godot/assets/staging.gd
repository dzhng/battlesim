extends RefCounted

## Brings repository assets into the project. Each file is linked on its own
## into the ignored `staging/` folder, mirroring its repository path, and one
## `--import` run imports whatever is new. Linking a directory instead would
## make Godot write `.import` files into the real `assets/` tree.
## A missing file, a Git LFS pointer or a failed import is a named blocker.

const STAGING := "res://staging"
const LFS_POINTER := "version https://git-lfs.github.com/spec/v1"

## An absolute path for `relative` to the repository root (or `relative` itself
## when it is already absolute).
static func repo_path(relative: String) -> String:
	if relative.is_absolute_path():
		return relative.simplify_path()
	return ProjectSettings.globalize_path("res://").path_join("../..").path_join(relative).simplify_path()

## Stages every repository-relative path in `relatives` and returns
## `{relative: resource path}` for those that imported. Every other path is a
## blocker in `report`.
static func stage(relatives: Array, report) -> Dictionary:
	var linked: Array[String] = []
	for relative in relatives:
		var source := repo_path(relative)
		if not FileAccess.file_exists(source):
			report.add_blocker("asset", "%s does not exist" % relative)
			continue
		var head := FileAccess.open(source, FileAccess.READ).get_buffer(LFS_POINTER.length()).get_string_from_utf8()
		if head == LFS_POINTER:
			report.add_blocker("asset", "%s is a Git LFS pointer; fetch it with git lfs pull --include=%s" % [relative, relative])
			continue
		var link := ProjectSettings.globalize_path(STAGING.path_join(relative))
		DirAccess.make_dir_recursive_absolute(link.get_base_dir())
		var directory := DirAccess.open(link.get_base_dir())
		if directory.is_link(link) and directory.read_link(link) != source:
			directory.remove(link)
		if not directory.is_link(link) and directory.create_link(source, link) != OK:
			report.add_blocker("asset", "could not link %s into %s" % [relative, STAGING])
			continue
		linked.append(relative)

	if linked.any(func(relative): return not FileAccess.file_exists(ProjectSettings.globalize_path(STAGING.path_join(relative)) + ".import")):
		var output := []
		var project := ProjectSettings.globalize_path("res://")
		var code := OS.execute(OS.get_executable_path(), ["--headless", "--path", project, "--import"], output, true)
		if code != 0:
			report.add_blocker("import", "godot --import exited %d: %s" % [code, "".join(output).right(2000)])

	var staged := {}
	for relative in linked:
		var resource := STAGING.path_join(relative)
		if ResourceLoader.exists(resource):
			staged[relative] = resource
		else:
			report.add_blocker("import", "%s did not import" % relative)
	return staged
