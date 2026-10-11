extends RefCounted

## The browser's combat effects for the parity run, as its battle session
## drives them (`useBattleSession.ts`): every publication up to a record's
## tick read as the effects read it (`effect_feed.gd`), with the hulls the
## side watched brew up (`cook_offs.gd`), noted in order (`effect_frame.gd`);
## each drawn record builds the effects at its `simTime` with the flashes on
## the muzzles the units layer drew (`drawn_models.gd`), lights the world and
## the smoke with their cast lights (`cast_lights.gd`) and hands the instances
## to the effect pass (`effect_pass.gd`).

const EffectFeed = preload("res://effects/effect_feed.gd")
const EffectFrame = preload("res://effects/effect_frame.gd")
const CookOffs = preload("res://effects/cook_offs.gd")
const CastLights = preload("res://effects/cast_lights.gd")
const DrawnModels = preload("res://effects/drawn_models.gd")
const EffectPass = preload("res://effects/effect_pass.gd")
const DetailView = preload("res://world/detail_view.gd")
const Coords = preload("res://core/coords.gd")
const StrictJson = preload("res://core/strict_json.gd")

const OWNER := "native/effects"

## The pass the environment draws them with, before its post chain.
var pass_: EffectPass
var _feed: EffectFeed
var _cook_offs: CookOffs
var _frame: EffectFrame
var _batch: EffectFrame.Batch
var _lights := CastLights.new()
var _drawn: DrawnModels
var _samples: Array
var _side: String
var _sun: Vector3
var _noted := 0
## Over the run: instances and lights drawn and dropped past the capacity.
var _drawn_instances := 0
var _dropped_instances := 0
var _dropped_lights := 0
## Cook-offs the side watched, whose wrecks' motion and debris native does
## not draw.
var _cook_off_count := 0

## `manifest` is the run's; `samples` the capture's decoded publications;
## `units` the units layer, whose drawn poses the flashes sit on (null: the
## flashes sit where the shots were published).
func _init(manifest: Dictionary, samples: Array, side: String, tick_hz: float, sky_lut, units) -> void:
	_samples = samples
	_side = side
	_feed = EffectFeed.new(manifest.units.types, manifest.effects)
	_cook_offs = CookOffs.new(manifest.units.types, manifest.effects, manifest.props, roundi(tick_hz))
	_frame = EffectFrame.new(tick_hz, manifest.effects.presentation)
	_batch = EffectFrame.Batch.new(_frame.capacity())
	_drawn = DrawnModels.new(units, side) if units != null else null
	_sun = Coords.sim_to_godot(StrictJson.vector3(manifest.light.sunDirection))
	pass_ = EffectPass.new(manifest.light, sky_lut, _frame.capacity())

## Notes every publication up to `record`'s tick, builds the effects at its
## clock as `camera` draws a viewport `size` pixels big, lights the world with
## them and hands them to the pass. Records come in order; the units layer
## has posed this one.
func present(record: Dictionary, camera: Camera3D, size: Vector2, units) -> void:
	while _noted < _samples.size() and int(_samples[_noted].tick) <= int(record.tick):
		var o: Dictionary = _samples[_noted]
		var pub := _feed.publication(o, _side)
		pub.cookOffs = _cook_offs.note(o)
		_cook_off_count += pub.cookOffs.size()
		_frame.note(pub)
		_noted += 1
	var muzzle := Callable()
	var rotors := Callable()
	if _drawn != null:
		_drawn.update(units.drawn)
		muzzle = _drawn.muzzle
		rotors = _drawn.rotors
	_frame.build(float(record.simTime), _batch, muzzle, rotors)
	_drawn_instances += _batch.count
	_dropped_instances += _batch.dropped
	_dropped_lights += _batch.light_dropped
	var view = DetailView.of(camera, size, _sun, 0.0)
	var target := StrictJson.vector3(record.target)
	var casts := _lights.pack(_batch, view, target, target.distance_to(StrictJson.vector3(record.eye)))
	pass_.near = camera.near
	pass_.far = camera.far
	pass_.set_instances(_batch.data, _batch.count, casts)

## What the last `present` drew (`EffectFrame.counts`).
func counts(record: Dictionary) -> Dictionary:
	return _frame.counts(float(record.simTime), _batch)

## Budget rows for the run: the instances and lights within their capacity,
## and what native draws differently from the browser.
func report_budgets(report) -> void:
	report.add_budget(OWNER, "effect instances drawn over the run, within %d a frame" % _frame.capacity(), "effects.instances", _drawn_instances + _dropped_instances, _drawn_instances, _dropped_instances)
	report.add_budget(OWNER, "cast-light candidates past the %d a frame offers" % EffectFrame.LIGHT_CANDIDATES, "effects.lights", _dropped_lights, 0, _dropped_lights)
	report.add_budget(OWNER, "approximation: lit smoke takes the sky averaged over each hemisphere, cosine-weighted; the browser reads its prefiltered environment at full roughness", "effects.sky", 1, 1, 0)
	report.add_budget(OWNER, "approximation: cast light on a model faces its mesh normal, not its normal map's", "effects.cast", 1, 1, 0)
	report.add_budget(OWNER, "not drawn: a cook-off's moving wreck (the hull rolling on and jolting, the turret thrown) and the debris it throws, which lies round the wreck for a while; the static wreck shows throughout", "effects.cookOffs", _cook_off_count, 0, _cook_off_count)
	var error := pass_.error()
	if not error.is_empty():
		report.add_blocker("effects", error)
