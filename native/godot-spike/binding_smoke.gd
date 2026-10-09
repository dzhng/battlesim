extends Node

func _ready() -> void:
	var probe := SimulationProbe.new()
	print(probe.binding_name(), " ", probe.simulation_crate_version())
	print("invalid scenario rejected: ", not probe.load_scenario("{}", 1))
	var args := OS.get_cmdline_user_args()
	if args.size() > 0:
		var scenario_path := args[0]
		var scenario := FileAccess.get_file_as_string(scenario_path)
		assert(probe.load_scenario(scenario, 11))
		var before := probe.digest()
		var publication := probe.publish_side("blue")
		assert(publication.size() > 0)
		assert(probe.accept_command("{\"side\":\"blue\",\"seq\":1,\"order\":{\"kind\":\"stop\",\"units\":[0]},\"queued\":false}"))
		assert(probe.step_fixed() == 1)
		var after := probe.digest()
		assert(before != after)
		var replay := probe.replay_json()
		assert(probe.load_replay(replay))
		assert(probe.step_fixed() == 1)
		assert(probe.digest() == after)
		print(JSON.stringify({"candidate":"godot-gdextension", "tick":probe.tick(), "publication_floats":publication.size(), "digest":after, "replay_digest_match":true}))
	probe.free()
	get_tree().quit()
