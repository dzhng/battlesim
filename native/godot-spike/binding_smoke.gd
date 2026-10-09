extends Node

func _ready() -> void:
	var probe := SimulationProbe.new()
	print(probe.binding_name(), " ", probe.simulation_crate_version())
	get_tree().quit()

