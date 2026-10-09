extends Node

func _ready() -> void:
	var probe := SimulationProbe.new()
	print(probe.binding_name(), " ", probe.simulation_crate_version())
	print("invalid scenario rejected: ", not probe.load_scenario("{}", 1))
	get_tree().quit()

