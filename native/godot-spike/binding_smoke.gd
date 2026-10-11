extends Node

# The runner also requires the completion record: Godot can exit zero after a script error.
func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	assert(args.size() == 2, "scenario and native evidence directory required")
	var scenario := FileAccess.get_file_as_string(args[0])
	var evidence: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(args[1] + "/expected.json"))
	var probe := SimulationProbe.new()
	add_child(probe)
	assert(not probe.load_scenario("{}", 1))
	assert(probe.load_scenario(scenario, int(evidence.seed)))
	assert(probe.accept_command("{\"side\":\"blue\",\"seq\":1,\"order\":{\"kind\":\"stop\",\"units\":[0]},\"queued\":false}"))
	assert(not probe.accept_command("{\"side\":\"blue\",\"seq\":1,\"order\":{\"kind\":\"stop\",\"units\":[0]},\"queued\":false}"), "duplicate sequence accepted")
	var retained := PackedFloat32Array()
	var retained_bytes := PackedByteArray()
	var step_us := 0
	var transfer_us := 0
	var bytes := 0
	for frame in evidence.frames:
		var start := Time.get_ticks_usec()
		assert(probe.step_fixed() == int(frame.tick))
		step_us += Time.get_ticks_usec() - start
		assert(probe.digest() == frame.digest, "native digest differs")
		start = Time.get_ticks_usec()
		var record := probe.publish_side(evidence.side)
		transfer_us += Time.get_ticks_usec() - start
		var actual := record.to_byte_array()
		assert(actual == FileAccess.get_file_as_bytes(args[1] + "/" + frame.file), "native publication differs")
		assert(retained.to_byte_array() == retained_bytes, "retained publication mutated")
		retained = record
		retained_bytes = actual
		bytes += actual.size()
	var digest := probe.digest()
	var replay := probe.replay_json()
	assert(JSON.parse_string(replay).accepted.size() == 1)
	assert(probe.load_replay(replay))
	assert(probe.tick() == 0)
	assert(not probe.accept_command("{\"side\":\"blue\",\"seq\":1,\"order\":{\"kind\":\"stop\",\"units\":[0]},\"queued\":false}"), "replay accepted live input")
	for frame in evidence.frames:
		assert(probe.step_fixed() == int(frame.tick))
	assert(probe.digest() == digest, "replay digest differs")
	var start := Time.get_ticks_usec()
	probe.free()
	print(JSON.stringify({"candidate":"godot-gdextension", "side":evidence.side, "digest":digest, "ticks":evidence.frames.size(), "bytes":bytes, "step_us":step_us, "publish_and_copy_us":transfer_us, "teardown_us":Time.get_ticks_usec()-start, "parity":true}))
	get_tree().quit()
