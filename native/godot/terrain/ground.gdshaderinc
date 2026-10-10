// The browser's ground formulas (`packages/battle-renderer/src/frame/
// terrainMaterial.ts`), ported as text: where a point lies in the plots,
// paving, forests and water, what colour and roughness the ground has there,
// and the class view's bytes. Every point is in sim metres (x east, y north).
//
// The tables are the browser's uploaded words (`core/data_texture.gd`), and
// the shader that includes this defines, from the manifest (`terrain_layer.gd`
// `shader_header`): each record field's word (`PLOTS_rows`), each record's
// stride, the surface index's constants (`SURFACEINDEX_kindShift`), each
// params field's vec4 (`PARAMS_verge`; a road's fields for road 0, with
// `PARAMS_roads_stride` and `PARAMS_roads_count`) and every tuning number
// (`TUNING_mottle_patchCut`, `TUNING_grain_weights_0`). Numbers that are part
// of a formula (hashes, lattice turns, decorrelation offsets, Rec. 709) stay
// here as text, as they do in the browser.

uniform sampler2D ground_nodes : filter_nearest;
uniform sampler2D ground_plots : filter_nearest;
uniform sampler2D ground_surfaces : filter_nearest;
uniform sampler2D ground_index : filter_nearest;
uniform vec4 ground_params[PARAMS_vec4s];

#define ROAD_KINDS uint(PARAMS_roads_count)
#define MAX_PLOT_DEPTH 64
#define CUT_A 1u
#define CUT_B 2u
#define LUMA vec3(0.2126, 0.7152, 0.0722)

// Tables ------------------------------------------------------------------------

uint table_word(sampler2D table, int i) {
	int width = textureSize(table, 0).x;
	uvec4 b = uvec4(round(texelFetch(table, ivec2(i % width, i / width), 0) * 255.0));
	return b.r | (b.g << 8u) | (b.b << 16u) | (b.a << 24u);
}

vec4 table_vec4(sampler2D table, int i) {
	return vec4(uintBitsToFloat(table_word(table, i)), uintBitsToFloat(table_word(table, i + 1)), uintBitsToFloat(table_word(table, i + 2)), uintBitsToFloat(table_word(table, i + 3)));
}

uint index_word(uint i) {
	return table_word(ground_index, int(i));
}

vec4 surface_field(uint record, int field) {
	return table_vec4(ground_surfaces, int(record) * SURFACES_stride + field);
}

vec4 plot_field(int plot, int field) {
	return table_vec4(ground_plots, plot * PLOTS_stride + field);
}

vec4 road(uint kind, int field) {
	return ground_params[field + int(kind) * PARAMS_roads_stride];
}

// Shared math --------------------------------------------------------------------

// WGSL's smoothstep, defined for any edges (GLSL's is not when low >= high).
float ss(float low, float high, float x) {
	float t = clamp((x - low) / (high - low), 0.0, 1.0);
	return t * t * (3.0 - 2.0 * t);
}

vec2 ss2(vec2 low, vec2 high, vec2 x) {
	vec2 t = clamp((x - low) / (high - low), 0.0, 1.0);
	return t * t * (3.0 - 2.0 * t);
}

vec4 ss4(vec4 low, vec4 high, vec4 x) {
	vec4 t = clamp((x - low) / (high - low), 0.0, 1.0);
	return t * t * (3.0 - 2.0 * t);
}

float saturate1(float x) {
	return clamp(x, 0.0, 1.0);
}

// The one integer hash (`shaders/pcgHash.ts`).
uint pcg_hash(uint v) {
	uint s = v * 747796405u + 2891336453u;
	uint w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
	return (w >> 22u) ^ w;
}

// Value noise on a unit lattice, in [0, 1] (`valueNoise`).
float value_noise(vec2 p) {
	vec2 cell = floor(p);
	vec2 f = p - cell;
	vec2 u = f * f * (3.0 - 2.0 * f);
	uint ix = uint(int(cell.x));
	uint iy = uint(int(cell.y));
	float h[4];
	for (int corner = 0; corner < 4; corner++) {
		uint k = uint(corner);
		uint s = pcg_hash(((ix + (k & 1u)) * 1597334677u) ^ ((iy + (k >> 1u)) * 3812015801u));
		h[corner] = float(s) * (1.0 / 4294967295.0);
	}
	return mix(mix(h[0], h[1], u.x), mix(h[2], h[3], u.x), u.y);
}

float rect_inside(vec2 xy, vec4 r) {
	return min(min(xy.x - r.x, r.z - xy.x), min(xy.y - r.y, r.w - xy.y));
}

float polygon_edge_distance(vec2 p, vec2 a, vec2 b) {
	vec2 ab = b - a;
	float len2 = dot(ab, ab);
	float t = 0.0;
	if (len2 > 0.0) {
		t = clamp(dot(p - a, ab) / len2, 0.0, 1.0);
	}
	return length(p - (a + ab * t));
}

float stroke_cut_inside(vec2 xy, vec4 ends, vec4 detail, float inside) {
	uint cuts = uint(detail.z);
	if (cuts == 0u) {
		return inside;
	}
	vec2 a = ends.xy;
	vec2 ab = ends.zw - a;
	float len = length(ab);
	float along = dot(xy - a, ab) / len;
	float past = max((cuts & CUT_A) != 0u ? -along : -1e9, (cuts & CUT_B) != 0u ? along - len : -1e9);
	if (past <= 0.0) {
		return min(inside, -past);
	}
	vec2 rel = xy - a;
	float aside = max(abs(rel.x * ab.y - rel.y * ab.x) / len - detail.x, 0.0);
	return -length(vec2(aside, past));
}

bool polygon_triangle_inside(vec2 xy, vec2 a, vec2 b, vec2 c) {
	vec2 ab = b - a;
	vec2 bc = c - b;
	vec2 ca = a - c;
	float area = dot(vec2(-ab.y, ab.x), c - a);
	float s0 = dot(vec2(-ab.y, ab.x), xy - a);
	float s1 = dot(vec2(-bc.y, bc.x), xy - b);
	float s2 = dot(vec2(-ca.y, ca.x), xy - c);
	return area != 0.0 && ((min(s0, min(s1, s2)) >= 0.0) || (max(s0, max(s1, s2)) <= 0.0));
}

// The surface field's cell under `xy` for a pixel `footprint` metres wide:
// where its paved, forest and water lists start and end (`groundCell`).
uvec4 ground_cell(vec2 xy, float footprint) {
	vec4 field = ground_params[PARAMS_field];
	uvec4 grid = uvec4(ground_params[PARAMS_fieldGrid]);
	uint level = 0u;
	float widest = field.w;
	while (footprint > widest && level + 1u < grid.z) {
		widest *= 2.0;
		level++;
	}
	uint shift = index_word(level * SURFACEINDEX_levelWords + 1u);
	ivec2 finest = clamp(ivec2(floor((xy - field.xy) * field.z)), ivec2(0), ivec2(grid.xy) - ivec2(1));
	uvec2 cell = uvec2(finest) >> uvec2(shift);
	uint cols = ((grid.x - 1u) >> shift) + 1u;
	uint at = index_word(level * SURFACEINDEX_levelWords) + (cell.y * cols + cell.x) * SURFACEINDEX_cellWords;
	return uvec4(index_word(at), index_word(at + 1u), index_word(at + 2u), index_word(at + 3u));
}

// Forests ------------------------------------------------------------------------

// How far `xy` lies inside the deepest forest shape of `cell` (`groundForest`).
float ground_forest(vec2 xy, uvec4 cell) {
	float forest = -1e9;
	float distance = -1e9;
	float nearest = 1e9;
	bool inside = false;
	for (uint i = cell.y; i < cell.z; i++) {
		uint entry = index_word(i);
		uint record = entry & SURFACEINDEX_recordMask;
		vec4 ends = surface_field(record, SURFACES_ends);
		vec4 detail = surface_field(record, SURFACES_detail);
		uint kind = entry >> SURFACEINDEX_kindShift;
		if ((entry & SURFACEINDEX_newShape) != 0u) {
			forest = max(forest, max(distance, inside ? nearest : -nearest));
			distance = -1e9;
			nearest = 1e9;
			inside = false;
		}
		if (kind == SURFACEINDEX_rect) {
			forest = max(forest, rect_inside(xy, ends));
		} else if (kind == SURFACEINDEX_stroke) {
			distance = max(distance, stroke_cut_inside(xy, ends, detail, detail.x - polygon_edge_distance(xy, ends.xy, ends.zw)));
		} else if (kind == SURFACEINDEX_triangle) {
			inside = inside || polygon_triangle_inside(xy, ends.xy, ends.zw, detail.xy);
		} else {
			nearest = min(nearest, polygon_edge_distance(xy, ends.xy, ends.zw));
		}
	}
	return max(forest, max(distance, inside ? nearest : -nearest));
}

// How far `xy` lies inside the forest floor's drawn edge (`forestVergeInside`).
float forest_verge_inside(vec2 xy, float forest) {
	vec4 verge = ground_params[PARAMS_forestVerge];
	float wander = (value_noise(xy * verge.z + vec2(71.3, 23.9)) - 0.5) * 2.0;
	float patches = value_noise(xy * TUNING_forestVerge_patchScale + vec2(5.1, 91.7)) - 0.5;
	return forest + verge.x * TUNING_forestVerge_inset + wander * verge.y + patches * verge.x;
}

float forest_floor_weight(vec2 xy, float forest, float footprint) {
	float feather = max(footprint, ground_params[PARAMS_forestVerge].x * TUNING_forestVerge_feather);
	return ss(-feather, feather, forest_verge_inside(xy, forest));
}

// The tree line at `xy`: how far inside the deepest strip, and inside its
// tapered band (`groundTreeLine`).
vec2 ground_tree_line(vec2 xy, uvec4 cell) {
	float inside = -1e9;
	float band = -1e9;
	float taper = ground_params[PARAMS_forestLine].x;
	for (uint i = cell.y; i < cell.z; i++) {
		uint entry = index_word(i);
		if ((entry >> SURFACEINDEX_kindShift) != SURFACEINDEX_stroke) {
			continue;
		}
		uint record = entry & SURFACEINDEX_recordMask;
		vec4 ends = surface_field(record, SURFACES_ends);
		vec4 detail = surface_field(record, SURFACES_detail);
		float here = stroke_cut_inside(xy, ends, detail, detail.x - polygon_edge_distance(xy, ends.xy, ends.zw));
		inside = max(inside, here);
		uint cuts = uint(detail.z);
		float narrow = here;
		if (cuts != 0u) {
			vec2 a = ends.xy;
			vec2 ab = ends.zw - a;
			float len = length(ab);
			vec2 rel = xy - a;
			float along = dot(rel, ab) / len;
			float end = min((cuts & CUT_A) != 0u ? along : 1e9, (cuts & CUT_B) != 0u ? len - along : 1e9);
			narrow = min(here, detail.x * min(1.0, end / taper) - abs(rel.x * ab.y - rel.y * ab.x) / len);
		}
		band = max(band, narrow);
	}
	return vec2(inside, band);
}

// Whether the forest at `xy` is a tree line, and its band (`groundLineBand`).
vec2 ground_line_band(vec2 xy, uvec4 cell, float forest) {
	vec2 line = ground_tree_line(xy, cell);
	if (line.x < forest) {
		return vec2(0.0, -1e9);
	}
	vec2 at = xy * ground_params[PARAMS_forestVerge].z + vec2(71.3, 23.9);
	return vec2(1.0, line.y + (value_noise(at) - 0.5) * 2.0 * ground_params[PARAMS_forestLine].y);
}

// The weight of a wood's floor (x) and a tree line's band (y) (`groundFloor`).
vec2 ground_floor(vec2 xy, float footprint, float forest) {
	vec4 verge = ground_params[PARAMS_forestVerge];
	float wood = forest_floor_weight(xy, forest, footprint);
	if (wood > 0.0 || forest > -verge.x - verge.y) {
		vec2 band = ground_line_band(xy, ground_cell(xy, footprint), forest);
		if (band.x > 0.5) {
			float feather = max(footprint, verge.x * TUNING_forestVerge_feather);
			return vec2(0.0, ss(-feather, feather, band.y));
		}
	}
	return vec2(wood, 0.0);
}

// The forest floor's linear albedo (`forestFloor`).
vec3 forest_floor(vec2 xy, float footprint, float noise) {
	vec4 detail = ground_params[PARAMS_forestDetail];
	vec4 verge = ground_params[PARAMS_forestVerge];
	vec2 at = xy * detail.x;
	vec2 turned = vec2(at.x * 0.8 - at.y * 0.6, at.x * 0.6 + at.y * 0.8);
	vec2 across = vec2(at.x * 0.28 + at.y * 0.96, at.y * 0.28 - at.x * 0.96);
	float moss = ss(TUNING_forestFloor_moss_cut_0, TUNING_forestFloor_moss_cut_1,
		value_noise(turned + vec2(13.3, 7.7)) * TUNING_forestFloor_moss_weights_0
		+ value_noise(across * TUNING_forestFloor_moss_fineScale + vec2(4.1, 9.2)) * TUNING_forestFloor_moss_weights_1);
	float humus = ss(TUNING_forestFloor_humus_cut_0, TUNING_forestFloor_humus_cut_1,
		value_noise(across * TUNING_forestFloor_humus_scales_0 + vec2(2.9, 41.1)) * TUNING_forestFloor_humus_weights_0
		+ value_noise(turned * TUNING_forestFloor_humus_scales_1 + vec2(17.3, 5.9)) * TUNING_forestFloor_humus_weights_1);
	vec3 floor_colour = mix(ground_params[PARAMS_forestLitter].xyz, ground_params[PARAMS_forestMoss].xyz, moss * ground_params[PARAMS_forestMoss].w);
	floor_colour = mix(floor_colour, ground_params[PARAMS_forestHumus].xyz, humus * ground_params[PARAMS_forestHumus].w);
	vec2 roots_at = xy * verge.w;
	float ridge = abs(value_noise(roots_at + vec2(29.1, 3.3)) - 0.5);
	float runs = ss(TUNING_forestFloor_roots_runs_0, TUNING_forestFloor_roots_runs_1, value_noise(roots_at * TUNING_forestFloor_roots_runScale + vec2(8.3, 17.9)));
	float shown = 1.0 - ss(TUNING_forestFloor_roots_fade_0, TUNING_forestFloor_roots_fade_1, footprint * verge.w);
	float line = (1.0 - ss(TUNING_forestFloor_roots_line_0, TUNING_forestFloor_roots_line_1, ridge)) * runs;
	float root = line * shown + TUNING_forestFloor_roots_mean * (1.0 - shown);
	floor_colour = floor_colour * (1.0 - detail.w * root);
	return floor_colour * (1.0 + detail.y * noise);
}

// Paving -------------------------------------------------------------------------

// Where a point sits in the paving (`GroundPaved`).
struct GroundPaved {
	vec4 drawn;
	vec4 lane;
	vec4 run;
	float rule;
	uvec2 list;
	float bearing;
};

GroundPaved ground_paved(vec2 xy, uvec4 cell) {
	vec4 paved = vec4(-1e9);
	uint member = ROAD_KINDS;
	float nearest = 1e9;
	uint edge = 0u;
	vec4 lane = vec4(0.0);
	vec4 run = vec4(0.0, 0.0, 0.0, -1.0);
	float deepest = -1e9;
	float bearing = -1.0;
	for (uint i = cell.x; i < cell.y; i++) {
		uint entry = index_word(i);
		uint record = entry & SURFACEINDEX_recordMask;
		vec4 ends = surface_field(record, SURFACES_ends);
		vec4 detail = surface_field(record, SURFACES_detail);
		uint kind = entry >> SURFACEINDEX_kindShift;
		if (kind == SURFACEINDEX_stroke) {
			vec2 a = ends.xy;
			vec2 ab = ends.zw - a;
			float t = clamp(dot(xy - a, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0);
			vec2 away = xy - (a + ab * t);
			float off = length(away);
			int own = int(detail.y);
			float inside = stroke_cut_inside(xy, ends, detail, detail.x - off);
			paved[own] = max(paved[own], inside);
			vec2 walk = road(uint(own), PARAMS_roads_join).zw;
			if (walk.x > 0.0) {
				paved[int(walk.y)] = max(paved[int(walk.y)], inside + walk.x);
			}
			if (inside > deepest) {
				deepest = inside;
				lane = vec4(away / max(off, 1e-5), off, detail.x);
				float len = max(length(ab), 1e-6);
				run = vec4(ab / len, detail.w + t * len, float(own));
			}
		} else if (kind == SURFACEINDEX_triangle) {
			if (polygon_triangle_inside(xy, ends.xy, ends.zw, detail.xy) && uint(detail.z) <= member) {
				member = uint(detail.z);
				bearing = detail.w;
			}
		} else {
			float off = polygon_edge_distance(xy, ends.xy, ends.zw);
			if (off < nearest) {
				nearest = off;
				edge = uint(detail.x);
			}
		}
	}
	float rule = deepest;
	if (member < ROAD_KINDS) {
		paved[int(member)] = max(paved[int(member)], nearest);
		rule = max(rule, nearest);
	} else if (nearest < 1e9) {
		paved[int(edge)] = max(paved[int(edge)], -nearest);
		rule = max(rule, -nearest);
	}
	return GroundPaved(paved, lane, run, rule, cell.xy, bearing);
}

// Where `xy` sits in the ground's features: `(plot, edge, road, forest)`
// (`groundSite`).
vec4 ground_site(vec2 xy, uvec4 cell, GroundPaved paved) {
	vec4 shape = ground_params[PARAMS_shape];
	vec2 warp = vec2(value_noise(xy * shape.y) - 0.5, value_noise(xy * shape.y + vec2(19.7, 5.3)) - 0.5);
	vec2 q = xy + warp * (shape.x * 2.0);
	float edge = rect_inside(q, ground_params[PARAMS_region]);
	int node = 0;
	int leaf = 0;
	for (int step = 0; step < MAX_PLOT_DEPTH; step++) {
		vec4 line = table_vec4(ground_nodes, node * NODES_stride + NODES_line);
		int s_child_x = int(table_word(ground_nodes, node * NODES_stride + NODES_children));
		int s_child_y = int(table_word(ground_nodes, node * NODES_stride + NODES_children + 1));
		float s = dot(line.xy, q) - line.z;
		edge = min(edge, abs(s));
		int child = s >= 0.0 ? s_child_x : s_child_y;
		if (child < 0) {
			leaf = -1 - child;
			break;
		}
		node = child;
	}
	return vec4(float(leaf), edge, paved.rule, ground_forest(xy, cell));
}

// Water --------------------------------------------------------------------------

// How far `xy` lies inside the water's edge (`groundWater`).
float ground_water(vec2 xy, uvec4 cell) {
	float bed = -1e9;
	for (uint i = cell.z; i < cell.w; i++) {
		uint record = index_word(i) & SURFACEINDEX_recordMask;
		vec4 ends = surface_field(record, SURFACES_ends);
		vec4 detail = surface_field(record, SURFACES_detail);
		vec2 a = ends.xy;
		vec2 ab = ends.zw - a;
		float len2 = dot(ab, ab);
		float t = 0.0;
		if (len2 > 0.0) {
			t = clamp(dot(xy - a, ab) / len2, 0.0, 1.0);
		}
		float half_width = detail.x + (detail.y - detail.x) * t;
		bed = max(bed, half_width - length(xy - (a + ab * t)));
	}
	return bed;
}

// A river bank's slope as shading reads it, and its weight (`groundBank`).
vec3 ground_bank(vec2 xy, uvec4 cell, float footprint) {
	vec4 sum = vec4(0.0);
	for (uint i = cell.z; i < cell.w; i++) {
		uint at = index_word(i) & SURFACEINDEX_recordMask;
		vec4 ends = surface_field(at, SURFACES_ends);
		vec4 detail = surface_field(at, SURFACES_detail);
		vec2 a = ends.xy;
		vec2 ab = ends.zw - a;
		float len2 = dot(ab, ab);
		float t = 0.0;
		if (len2 > 0.0) {
			t = clamp(dot(xy - a, ab) / len2, 0.0, 1.0);
		}
		vec2 away = xy - (a + ab * t);
		float off = length(away);
		float half_width = detail.x + (detail.y - detail.x) * t;
		float inside = half_width - off;
		float grade = detail.z + (detail.w - detail.z) * t;
		vec2 heights = surface_field(at + 1u, SURFACES_ends).xy;
		float run = (heights.x + (heights.y - heights.x) * t) / max(grade, 1e-4);
		float ease = max(footprint, TUNING_bank_easeM);
		float sloped = (1.0 - ss(run - ease, run + ease, -inside)) * (1.0 - ss(0.0, ease, inside));
		float top = run + ground_params[PARAMS_waterBed].w;
		float weight = exp(clamp(inside, -TUNING_bank_blendReachM, TUNING_bank_blendReachM) / TUNING_bank_blendM);
		float shown = min(grade * ground_params[PARAMS_shoreEdge].w, TUNING_shading_maxSlope);
		sum += vec4(away / max(off, 1e-4) * shown * sloped, 1.0 - ss(top, top + 2.0 * ease, -inside), 1.0) * weight;
	}
	if (sum.w <= 0.0) {
		return vec3(0.0);
	}
	return sum.xyz / sum.w;
}

vec2 turned(vec2 xy, float turn) {
	float c = cos(turn);
	float s = sin(turn);
	return vec2(xy.x * c + xy.y * s, xy.y * c - xy.x * s);
}

// The shore's bands `(wet, earth, bare)` (`groundShore`).
vec3 ground_shore(vec2 xy, float footprint, float water) {
	float out_m = -water;
	float wet = ground_params[PARAMS_shore].w;
	float reach = ground_params[PARAMS_shoreEarth].w;
	if (out_m >= reach) {
		return vec3(0.0);
	}
	vec4 edge = ground_params[PARAMS_shoreEdge];
	float broad = value_noise(turned(xy, TUNING_shore_turns_0) * edge.y);
	float scale = edge.y * TUNING_shore_fine;
	float fine = mix(value_noise(turned(xy, TUNING_shore_turns_1) * scale + vec2(41.7, 13.1)), 0.5, ss(TUNING_shore_fineFade_0, TUNING_shore_fineFade_1, footprint * scale));
	float line = reach * (1.0 - edge.x * mix(broad, fine, TUNING_shore_fineShare));
	return vec3(
		1.0 - ss(wet * TUNING_shore_wetWhole, wet, out_m),
		1.0 - ss(mix(wet, line, TUNING_shore_earthWhole), line, out_m),
		1.0 - ss(wet, line, out_m));
}

vec3 at_least(vec3 colour, float least) {
	float luminance = dot(colour, LUMA);
	return colour * max(1.0, least / max(luminance, 1e-5));
}

// The verge along every plot edge (`groundVerge`).
float ground_verge(vec4 site, float footprint) {
	float verge_half = ground_params[PARAMS_verge].w;
	return 1.0 - ss(verge_half, verge_half + max(ground_params[PARAMS_feathers].x, footprint), site.y);
}

// Roads --------------------------------------------------------------------------

float wander_noise(vec2 at) {
	vec2 push = vec2(value_noise(at * TUNING_wander_pushScale + vec2(11.3, 47.1)), value_noise(at * TUNING_wander_pushScale + vec2(83.9, 5.7))) - 0.5;
	vec2 p = at + push * TUNING_wander_push;
	float a = value_noise(vec2(0.87 * p.x - 0.5 * p.y, 0.5 * p.x + 0.87 * p.y));
	float b = value_noise(vec2(0.6 * p.x + 0.8 * p.y, -0.8 * p.x + 0.6 * p.y) * TUNING_wander_octaves_0 + vec2(19.1, 7.7));
	float c = value_noise(vec2(0.96 * p.x - 0.28 * p.y, 0.28 * p.x + 0.96 * p.y) * TUNING_wander_octaves_1 + vec2(3.3, 41.9));
	return a * TUNING_wander_weights_0 + b * TUNING_wander_weights_1 + c * TUNING_wander_weights_2;
}

float road_grain(vec2 p, float pixel) {
	vec4 shown = vec4(1.0) - ss4(vec4(TUNING_roadGrain_fade_0), vec4(TUNING_roadGrain_fade_1), pixel * vec4(1.0, TUNING_roadGrain_octaves_0, TUNING_roadGrain_octaves_1, TUNING_roadGrain_octaves_2));
	float grain = 0.0;
	if (shown.x > 0.0) {
		grain += (value_noise(vec2(0.87 * p.x - 0.5 * p.y, 0.5 * p.x + 0.87 * p.y)) - 0.5) * TUNING_roadGrain_weights_0 * shown.x;
	}
	if (shown.y > 0.0) {
		grain += (value_noise(vec2(0.6 * p.x + 0.8 * p.y, -0.8 * p.x + 0.6 * p.y) * TUNING_roadGrain_octaves_0 + vec2(19.1, 7.7)) - 0.5) * TUNING_roadGrain_weights_1 * shown.y;
	}
	if (shown.z > 0.0) {
		grain += (value_noise(vec2(0.96 * p.x - 0.28 * p.y, 0.28 * p.x + 0.96 * p.y) * TUNING_roadGrain_octaves_1 + vec2(3.3, 41.9)) - 0.5) * TUNING_roadGrain_weights_2 * shown.z;
	}
	if (shown.w > 0.0) {
		float stone = value_noise(p * TUNING_roadGrain_octaves_2 + vec2(71.9, 13.7));
		grain += (ss(TUNING_roadGrain_stones_0, TUNING_roadGrain_stones_1, stone) - ss(TUNING_roadGrain_stones_0, TUNING_roadGrain_stones_1, 1.0 - stone)) * TUNING_roadGrain_weights_3 * shown.w;
	}
	return grain;
}

bool roads_plain() {
	return uint(ground_params[PARAMS_view].y) == 1u;
}

// The kind of the stroke whose lanes run under a point, or -1 (`roadLane`).
int road_lane(GroundPaved paved) {
	if (paved.run.w < 0.0) {
		return -1;
	}
	uint own = uint(paved.run.w);
	if (paved.drawn[int(own)] <= 0.0) {
		return -1;
	}
	vec4 order = ground_params[PARAMS_roadOrder];
	for (uint n = ROAD_KINDS; n > 0u; n--) {
		uint k = uint(order[int(n - 1u)]);
		if (k == own) {
			break;
		}
		if (paved.drawn[int(k)] > 0.0) {
			return -1;
		}
	}
	return int(own);
}

// A track's grass centre strip (`groundStrip`).
float ground_strip(vec2 xy, float footprint, GroundPaved paved) {
	int kind = road_lane(paved);
	if (kind < 0 || roads_plain()) {
		return 0.0;
	}
	vec4 track = road(uint(kind), PARAMS_roads_track);
	float half_width = track.y;
	if (half_width <= 0.0 || paved.lane.w > track.z) {
		return 0.0;
	}
	float reach = half_width * (1.0 + (wander_noise(xy * (1.0 / TUNING_strip_wanderM) + vec2(41.3, 9.7)) - 0.5) * TUNING_strip_wander);
	float soft = max(footprint, TUNING_strip_edgeM);
	float shown = 1.0 - ss(TUNING_strip_pixels_0, TUNING_strip_pixels_1, footprint / (2.0 * half_width));
	return (1.0 - ss(reach - soft, reach + soft, paved.lane.z)) * shown;
}

// A rut's cross-section `(1 - u * u)^2`: its mean, and its steepest side.
#define RUT_MEAN (8.0 / 15.0)
#define RUT_STEEPEST 1.5396

// The ruts: `(slope east, slope north, shade)` (`groundRuts`).
vec3 ground_ruts(vec2 xy, float footprint, GroundPaved paved) {
	int kind = road_lane(paved);
	if (kind < 0 || roads_plain()) {
		return vec3(0.0);
	}
	vec4 ruts_look = road(uint(kind), PARAMS_roads_ruts);
	float width = ruts_look.z;
	if (width <= 0.0) {
		return vec3(0.0);
	}
	float shown = 1.0 - ss(TUNING_ruts_fade_0, TUNING_ruts_fade_1, footprint / width);
	if (shown <= 0.0) {
		return vec3(0.0);
	}
	float half_width = paved.lane.w;
	float depth = 0.0;
	float rise = 0.0;
	float ruts = 0.0;
	for (int i = 0; i < 2; i++) {
		float at = ruts_look[i];
		if (at <= 0.0 || at + width > half_width) {
			continue;
		}
		ruts += 2.0;
		float u = (paved.lane.z - at) / (width * 0.5);
		if (abs(u) < 1.0) {
			float b = 1.0 - u * u;
			depth = b * b;
			rise = 4.0 * u * b;
		}
	}
	if (ruts == 0.0) {
		return vec3(0.0);
	}
	float faint = mix(TUNING_ruts_faint, 1.0, ss(TUNING_ruts_breakCut_0, TUNING_ruts_breakCut_1, wander_noise(xy * (1.0 / TUNING_ruts_breakM) + vec2(13.9, 57.3)))) * shown;
	float mean = ruts * width * RUT_MEAN / (2.0 * half_width);
	return vec3(paved.lane.xy * (ruts_look.w * rise * (1.0 / RUT_STEEPEST) * faint), road(uint(kind), PARAMS_roads_track).x * (mean - depth) * faint);
}

// How worn the ground is by the roads, and by which kind: `(wear, kind)`
// (`groundShoulder`).
vec2 ground_shoulder(vec2 xy, float footprint, GroundPaved paved) {
	vec2 worn = vec2(0.0);
	bool plain = roads_plain();
	for (uint k = 0u; k < ROAD_KINDS; k++) {
		vec4 shoulder = road(k, PARAMS_roads_shoulder);
		vec4 edge = road(k, PARAMS_roads_edge);
		float width = shoulder.w;
		float out_m = -paved.drawn[int(k)];
		if (out_m <= 0.0) {
			worn = vec2(1.0, float(k));
			continue;
		}
		if (plain || out_m >= width) {
			continue;
		}
		float wander = saturate1((wander_noise(xy * edge.y + vec2(27.3, 88.1)) - 0.5) * TUNING_shoulder_wander + 0.5);
		float reach = width * (1.0 - edge.x * wander);
		float across = saturate1(out_m / reach);
		float tufts = 1.0 - across * (1.0 - ss(TUNING_shoulder_tufts_0, TUNING_shoulder_tufts_1, wander_noise(xy * (1.0 / TUNING_shoulder_tuftM) + vec2(5.9, 63.1))));
		float wear = (1.0 - ss(TUNING_shoulder_whole, 1.0, out_m / reach)) * tufts * (1.0 - ss(TUNING_shoulder_pixels_0, TUNING_shoulder_pixels_1, footprint / width));
		if (wear > worn.x) {
			worn = vec2(wear, float(k));
		}
	}
	float strip = ground_strip(xy, footprint, paved);
	if (strip > 0.0) {
		worn = vec2(1.0 - strip * TUNING_strip_grass, float(road_lane(paved)));
	}
	return worn;
}

float stroke_joint(float along, float per_metre, float footprint, float width) {
	float shown = 1.0 - ss(TUNING_slabs_jointPixels_0, TUNING_slabs_jointPixels_1, footprint / width);
	if (per_metre <= 0.0 || shown <= 0.0) {
		return 0.0;
	}
	float to = abs(fract(along * per_metre + 0.5) - 0.5) / per_metre;
	return (1.0 - ss(width * 0.5, width * 0.5 + footprint, to)) * shown;
}

float area_joint(float along, float per_metre, float footprint, float width) {
	float drawn = max(width, footprint);
	float to = abs(fract(along * per_metre + 0.5) - 0.5) / per_metre;
	return (1.0 - ss(drawn * 0.5 - footprint * 0.5, drawn * 0.5 + footprint * 0.5, to)) * (width / drawn);
}

float walk_joint(float footprint, GroundPaved paved) {
	if (paved.run.w < 0.0 || roads_plain()) {
		return 0.0;
	}
	uint own = uint(paved.run.w);
	float beyond = paved.lane.z - paved.lane.w;
	if (beyond <= 0.0 || beyond >= road(own, PARAMS_roads_join).z) {
		return 0.0;
	}
	vec4 slabs = road(own, PARAMS_roads_slabs);
	return stroke_joint(paved.run.z, slabs.x, footprint, TUNING_slabs_jointM) * slabs.y;
}

// The curb: `(slope east, slope north, stones)` (`groundCurb`).
vec3 ground_curb(float footprint, GroundPaved paved) {
	if (paved.run.w < 0.0 || roads_plain()) {
		return vec3(0.0);
	}
	uint own = uint(paved.run.w);
	vec4 curb = road(own, PARAMS_roads_curb);
	vec4 slabs = road(own, PARAMS_roads_slabs);
	float width = curb.w;
	if (width <= 0.0) {
		return vec3(0.0);
	}
	float inside = paved.drawn[int(own)];
	if (inside < -2.0 * width || inside > 2.0 * slabs.z) {
		return vec3(0.0);
	}
	float shown = 1.0 - ss(TUNING_curb_pixels_0, TUNING_curb_pixels_1, footprint / width);
	if (shown <= 0.0) {
		return vec3(0.0);
	}
	float soft = max(footprint, TUNING_curb_edgeM) * 0.5;
	float open = 1.0;
	for (uint k = 0u; k < ROAD_KINDS; k++) {
		if (k != own && road(k, PARAMS_roads_track).w > 0.0) {
			open *= 1.0 - ss(-soft, soft, paved.drawn[int(k)]);
		}
	}
	float edge = ss(-soft, soft, inside);
	float stones = ss(-width - soft, -width + soft, inside) * (1.0 - edge);
	float face = edge * (1.0 - ss(slabs.z - soft, slabs.z + soft, inside));
	return vec3(paved.lane.xy * (slabs.w * face), stones) * (open * shown);
}

// The slope a road's ruts and curb give the shading normal (`groundRoadRelief`).
vec2 ground_road_relief(vec2 xy, float footprint, GroundPaved paved) {
	return ground_ruts(xy, footprint, paved).xy + ground_curb(footprint, paved).xy;
}

// How much paint lies at `xy` (`groundMarks`).
float ground_marks(vec2 xy, float footprint, GroundPaved paved) {
	int kind = road_lane(paved);
	if (kind < 0 || roads_plain()) {
		return 0.0;
	}
	vec4 marks = road(uint(kind), PARAMS_roads_marks);
	float line = marks.x;
	if (line <= 0.0) {
		return 0.0;
	}
	vec4 bars = road(uint(kind), PARAMS_roads_crossing);
	vec2 shown = vec2(1.0) - ss2(vec2(TUNING_marks_pixels_0), vec2(TUNING_marks_pixels_1), footprint / vec2(2.0 * line, bars.x));
	if (shown.x <= 0.0 && shown.y <= 0.0) {
		return 0.0;
	}
	float far_m = bars.z + bars.y;
	float dash = (fract(paved.run.z / marks.z) - 0.5) * marks.z;
	float near_m = -1e9;
	float cross = -1e9;
	for (uint i = paved.list.x; i < paved.list.y; i++) {
		uint entry = index_word(i);
		if ((entry >> SURFACEINDEX_kindShift) != SURFACEINDEX_stroke) {
			continue;
		}
		uint record = entry & SURFACEINDEX_recordMask;
		vec4 ends = surface_field(record, SURFACES_ends);
		vec4 detail = surface_field(record, SURFACES_detail);
		if (road(uint(detail.y), PARAMS_roads_track).w <= 0.0) {
			continue;
		}
		vec2 a = ends.xy;
		vec2 ab = ends.zw - a;
		float len = max(length(ab), 1e-6);
		if (abs(dot(ab, paved.run.xy)) > TUNING_marks_crossingCos * len) {
			continue;
		}
		float free_m = dot(xy - a, ab) / len;
		vec2 away = xy - (a + ab * (clamp(free_m, 0.0, len) / len));
		float off = length(away);
		float inside = stroke_cut_inside(xy, ends, detail, detail.x - off);
		float nearer = dot(paved.run.xy, away) / max(off, 1e-5);
		near_m = max(near_m, inside + nearer * dash + abs(nearer) * marks.y * 0.5);
		uint cuts = uint(detail.z);
		float room = 2.0 * paved.lane.w;
		bool at_end = ((cuts & CUT_A) != 0u && free_m < room) || ((cuts & CUT_B) != 0u && len - free_m < room);
		if (!at_end && free_m >= 0.0 && free_m <= len) {
			cross = max(cross, inside);
		}
	}
	float soft = max(footprint, TUNING_marks_edgeM) * 0.5;
	float centre = (1.0 - ss(line - soft, line + soft, paved.lane.z))
		* (1.0 - ss(marks.y * 0.5 - soft, marks.y * 0.5 + soft, abs(dash)))
		* (1.0 - step(-far_m - TUNING_marks_clearM, near_m));
	float zone = ss(-far_m - soft, -far_m + soft, cross) * (1.0 - ss(-bars.z - soft, -bars.z + soft, cross));
	float across = abs(fract(paved.lane.z / (2.0 * bars.x)) - 0.5) * 2.0 * bars.x;
	float bar = 1.0 - ss(bars.x * 0.5 - soft, bars.x * 0.5 + soft, across);
	float kept = 1.0 - ss(paved.lane.w - bars.w - soft, paved.lane.w - bars.w + soft, paved.lane.z);
	return max(centre * shown.x, zone * bar * kept * shown.y);
}

// The roads at `xy` over the ground `under` (linear albedo, roughness)
// (`groundRoads`).
vec4 ground_roads(vec2 xy, float footprint, GroundPaved paved, vec4 under) {
	vec4 surface = under;
	bool plain = roads_plain();
	vec2 worn = ground_shoulder(xy, footprint, paved);
	if (worn.x > 0.0) {
		uint k = uint(worn.y);
		vec4 shoulder = road(k, PARAMS_roads_shoulder);
		vec4 shape = road(k, PARAMS_roads_shape);
		float lift = max(1.0, dot(under.xyz, LUMA) / max(dot(shoulder.xyz, LUMA), 1e-5));
		float grain = road_grain(xy * shape.w, footprint * shape.w) * TUNING_shoulder_grain;
		surface = mix(under, vec4(shoulder.xyz * lift * (1.0 + shape.z * grain), road(k, PARAMS_roads_core).w), worn.x * road(k, PARAMS_roads_edge).w);
	}
	int lane = road_lane(paved);
	vec4 order = ground_params[PARAMS_roadOrder];
	float carried = 0.0;
	for (uint n = 0u; n < ROAD_KINDS; n++) {
		uint k = uint(order[int(n)]);
		vec4 core_look = road(k, PARAMS_roads_core);
		vec4 shape = road(k, PARAMS_roads_shape);
		vec4 worn_look = road(k, PARAMS_roads_worn);
		vec4 stones_look = road(k, PARAMS_roads_stones);
		float feather = max(shape.x, footprint) * 0.5;
		float inside = paved.drawn[int(k)];
		if (carried > 0.0) {
			feather = max(feather, carried * 0.5);
			inside -= feather;
			if (!plain) {
				inside += (wander_noise(xy * (1.0 / TUNING_surface_joinDriftM) + vec2(7.9, 52.3)) - 0.5) * 2.0 * carried;
			}
		}
		if (inside <= -feather) {
			continue;
		}
		vec3 core = core_look.xyz;
		if (!plain) {
			float hue = ss(TUNING_surface_patchCut_0, TUNING_surface_patchCut_1, wander_noise(xy * shape.y + vec2(61.7, 17.3))) * worn_look.w;
			float grain = road_grain(xy * shape.w, footprint * shape.w);
			core = mix(core, worn_look.xyz, hue) * (1.0 + shape.z * grain);
			if (int(k) == lane) {
				core *= 1.0 + ground_ruts(xy, footprint, paved).z;
				float strip = ground_strip(xy, footprint, paved) * TUNING_strip_cover;
				core = mix(core, ground_params[PARAMS_verge].xyz * (1.0 + shape.z * grain * TUNING_shoulder_grain), strip);
			} else if (lane < 0) {
				float out_m = 1e9;
				if (paved.run.w >= 0.0) {
					vec4 join = road(uint(paved.run.w), PARAMS_roads_join);
					out_m = paved.lane.z - paved.lane.w - join.z;
					if (out_m < 0.0 && uint(join.w) == k) {
						core *= 1.0 - walk_joint(footprint, paved);
					}
				}
				if (out_m >= 0.0 && stones_look.z > 0.0 && paved.bearing >= 0.0 && !plain) {
					float border = 1.0 - ss(0.0, footprint, out_m - min(1.0 / stones_look.z, TUNING_slabs_areaBorderM));
					float w = TUNING_slabs_areaJointM;
					vec2 along = vec2(cos(paved.bearing), sin(paved.bearing));
					float near_grid = (1.0 - ss(TUNING_slabs_areaSlabPixels_0, TUNING_slabs_areaSlabPixels_1, footprint * stones_look.z)) * (1.0 - border);
					float joint = near_grid * max(area_joint(dot(xy, along), stones_look.z, footprint, w), area_joint(dot(xy, vec2(-along.y, along.x)), stones_look.z, footprint, w));
					float drawn = max(w, footprint);
					float edge = (1.0 - ss(drawn - footprint * 0.5, drawn + footprint * 0.5, out_m)) * (w / drawn);
					core *= 1.0 - max(joint, edge * TUNING_slabs_areaEdgeDepth) * stones_look.w;
				}
			}
		}
		float on = ss(-feather, feather, inside);
		surface = mix(surface, vec4(core, core_look.w), on);
		if (road(k, PARAMS_roads_track).w > 0.0) {
			carried = max(carried, road(k, PARAMS_roads_join).x * on);
		}
	}
	float paint = ground_marks(xy, footprint, paved);
	if (paint > 0.0) {
		uint k = uint(paved.run.w);
		vec4 marks = road(k, PARAMS_roads_marks);
		vec4 shape = road(k, PARAMS_roads_shape);
		vec4 paint_look = road(k, PARAMS_roads_paint);
		float worn_paint = 1.0 - marks.w * ss(TUNING_marks_wearCut_0, TUNING_marks_wearCut_1, wander_noise(xy * (1.0 / TUNING_marks_wearM) + vec2(33.7, 71.1)));
		float grain = road_grain(xy * shape.w, footprint * shape.w);
		surface = mix(surface, vec4(paint_look.xyz * (1.0 + shape.z * grain), road(k, PARAMS_roads_core).w), paint * paint_look.w * worn_paint);
	}
	float stones = ground_curb(footprint, paved).z;
	if (stones > 0.0) {
		uint k = uint(paved.run.w);
		vec4 shape = road(k, PARAMS_roads_shape);
		vec4 stones_look = road(k, PARAMS_roads_stones);
		float grain = road_grain(xy * shape.w, footprint * shape.w);
		float joint = stroke_joint(paved.run.z, stones_look.x, footprint, TUNING_slabs_jointM) * stones_look.y;
		surface = mix(surface, vec4(road(k, PARAMS_roads_curb).xyz * (1.0 + shape.z * grain) * (1.0 - joint), road(k, PARAMS_roads_core).w), stones);
	}
	return surface;
}

// The plot's own ground ----------------------------------------------------------

// The painterly variation of a plot's ground, `(value, dry)` (`mottle`):
// dry patches drawn afresh in every plot along its rows, with edges a pixel
// wide, and fine value noise that fades below a pixel.
vec2 ground_mottle(vec2 xy, float footprint, vec2 across, float plot, vec2 reach) {
	vec4 shape = ground_params[PARAMS_shape];
	vec2 along = vec2(-across.y, across.x);
	vec2 lane = vec2(dot(xy, along) * reach.x, dot(xy, across) * reach.y);
	vec2 seed = vec2(fract(plot * 0.6180339) * 997.0, fract(plot * 0.7548776) * 991.0);
	vec2 at = lane + seed;
	float streak = value_noise(at);
	vec2 slope = vec2(
		(value_noise(at + vec2(TUNING_mottle_slopeStep, 0.0)) - streak) / TUNING_mottle_slopeStep * reach.x,
		(value_noise(at + vec2(0.0, TUNING_mottle_slopeStep)) - streak) / TUNING_mottle_slopeStep * reach.y);
	float inside = (streak - TUNING_mottle_patchCut) / max(length(slope), 1e-4);
	float edge = max(footprint, TUNING_mottle_patchEdgeM) * 0.5;
	float parched = ss(-edge, edge, inside);
	float fine = value_noise(xy * shape.z + vec2(37.1, 11.3)) - 0.5;
	float fine_shown = 1.0 - ss(TUNING_mottle_fineFade_0, TUNING_mottle_fineFade_1, footprint * shape.z);
	return vec2(fine * TUNING_mottle_fineValue * fine_shown, parched * TUNING_mottle_patchHue + max(fine, 0.0) * TUNING_mottle_fineHue * fine_shown);
}

// How much of a pixel a wheeling of plot `plot` covers: two furrows bare in
// every `tram.x` (`groundWheeling`).
float ground_wheeling(vec2 xy, float footprint, int plot) {
	vec4 rows = plot_field(plot, PLOTS_rows);
	vec4 tram = plot_field(plot, PLOTS_tram);
	if (tram.x <= 0.0 || rows.z <= 0.0) {
		return 0.0;
	}
	float phase = dot(xy, rows.xy) / rows.z;
	float furrow = roundEven(phase);
	float nth = furrow - tram.x * floor(furrow / tram.x);
	if (nth != 0.0 && nth != TUNING_wheeling_gaugeRows) {
		return 0.0;
	}
	float off = abs(phase - furrow) * rows.z;
	float pixel = max(footprint, TUNING_wheeling_leastPixelM);
	return clamp((tram.y - off) / pixel + 0.5, 0.0, 1.0) * min(1.0, 2.0 * tram.y / pixel);
}

// What a plot's rows, wheelings and grain do to its albedo, as a factor
// (`fieldTexture`). Every term fades to its mean below a pixel.
float field_texture(vec2 xy, float footprint, int plot) {
	vec4 rows = plot_field(plot, PLOTS_rows);
	vec4 grain = plot_field(plot, PLOTS_grain);
	float breaks = plot_field(plot, PLOTS_detail).z;
	float u = dot(xy, vec2(-rows.y, rows.x));
	float v = dot(xy, rows.xy);
	float value = 1.0;
	if (rows.z > 0.0) {
		float phase = v / rows.z;
		float shown = 1.0 - ss(TUNING_rows_fade_0, TUNING_rows_fade_1, footprint / rows.z);
		float stripe = 0.5 + 0.5 * cos(phase * 6.2831853);
		float furrow = stripe * stripe * stripe;
		float along = grain.x * grain.z / TUNING_rows_breakGrains;
		float seen = shown * (1.0 - ss(TUNING_grain_fade_0, TUNING_grain_fade_1, footprint * along));
		if (breaks > 0.0 && seen > 0.0) {
			// Each furrow has its own noise along its length; two meet on the
			// bed between them, where neither darkens anything.
			float gap = value_noise(vec2(u * along, floor(phase + 0.5) * 7.31 + 0.5));
			furrow *= 1.0 + breaks * (gap * 2.0 - 1.0) * seen;
		}
		float bare = plot_field(plot, PLOTS_tram).z * ground_wheeling(xy, footprint, plot);
		value = (1.0 - rows.w * (0.5 + (furrow - TUNING_rows_mean) * shown)) * (1.0 - bare);
	}
	if (grain.y > 0.0) {
		float size = grain.x / (TUNING_grain_octave * TUNING_grain_octave);
		float sum = 0.0;
		vec2 push = vec2(0.0);
		float weights[4] = float[4](TUNING_grain_weights_0, TUNING_grain_weights_1, TUNING_grain_weights_2, TUNING_grain_weights_3);
		float turns[4] = float[4](TUNING_grain_turns_0, TUNING_grain_turns_1, TUNING_grain_turns_2, TUNING_grain_turns_3);
		for (int i = 0; i < 4; i++) {
			float shown = 1.0 - ss(TUNING_grain_fade_0, TUNING_grain_fade_1, footprint * size);
			if (shown > 0.0) {
				vec2 turn = vec2(cos(turns[i]), sin(turns[i]));
				vec2 p = vec2(u * grain.z, v) * size;
				vec2 seed = vec2(float(i) * 17.31 + 3.7, float(plot % 64) * 1.37);
				float n = value_noise(vec2(dot(p, turn), dot(p, vec2(-turn.y, turn.x))) + seed + push);
				sum += weights[i] * shown * clamp((n - 0.5) * (2.0 * TUNING_grain_crisp), -1.0, 1.0);
				push = vec2(cos(n * 6.2831853), sin(n * 6.2831853)) * TUNING_grain_warp;
			}
			size *= TUNING_grain_octave;
		}
		value *= 1.0 + grain.y * sum;
	}
	return value;
}

// `albedo` toward ochre by `dry` at its own luminance (`groundTint`).
vec3 ground_tint(vec3 albedo, float dry) {
	vec3 tinted = albedo * (vec3(1.0) + vec3(TUNING_mottle_dry_0, TUNING_mottle_dry_1, TUNING_mottle_dry_2) * dry);
	return tinted * (dot(albedo, LUMA) / max(dot(tinted, LUMA), 1e-5));
}

// The ground ---------------------------------------------------------------------

// Linear albedo and roughness of the ground at `xy` (`groundColour`).
vec4 ground_colour(vec2 xy, float footprint, vec4 site, GroundPaved paved, float water) {
	float aa = footprint * 0.5;
	int plot = int(site.x);
	vec4 colour = plot_field(plot, PLOTS_colour);
	vec4 rows = plot_field(plot, PLOTS_rows);
	vec4 detail = plot_field(plot, PLOTS_detail);
	vec2 variation = ground_mottle(xy, footprint, rows.xy, site.x, plot_field(plot, PLOTS_dry).xy);
	float noise = variation.x;
	vec3 albedo = ground_tint(colour.xyz * field_texture(xy, footprint, plot), detail.x * variation.y);
	float roughness = colour.w;

	vec3 verge_colour = ground_params[PARAMS_verge].xyz * (1.0 + TUNING_verge_mottle * noise);
	float verge = ground_verge(site, footprint);
	albedo = mix(albedo, verge_colour, verge);
	roughness = mix(roughness, TUNING_verge_roughness, verge);

	vec4 feathers = ground_params[PARAMS_feathers];
	float in_region = rect_inside(xy, ground_params[PARAMS_region]);
	float distant = max(1.0 - ss(0.0, TUNING_distant_fadeM, in_region), ss(feathers.y, feathers.z, footprint));
	albedo = mix(albedo, ground_params[PARAMS_distant].xyz, distant);

	vec2 floor_weight = ground_floor(xy, footprint, site.w);
	if (floor_weight.x > 0.0) {
		albedo = mix(albedo, forest_floor(xy, footprint, noise), floor_weight.x);
		roughness = mix(roughness, ground_params[PARAMS_forestDetail].z, floor_weight.x);
	}
	if (floor_weight.y > 0.0) {
		albedo = mix(albedo, verge_colour, floor_weight.y);
		roughness = mix(roughness, TUNING_verge_roughness, floor_weight.y);
	}

	vec3 shore = ground_shore(xy, footprint, water);
	if (shore.y > 0.0) {
		float field = mix(dot(ground_params[PARAMS_verge].xyz, LUMA), dot(colour.xyz, LUMA) * (1.0 - 0.5 * rows.w), ss(0.0, TUNING_shore_fieldEaseM, site.y));
		float least = field * ground_params[PARAMS_shoreEdge].z;
		vec3 earth = ground_params[PARAMS_shoreEarth].xyz * (1.0 + TUNING_shore_earthMottle * noise);
		albedo = mix(albedo, at_least(earth, least), shore.y);
		roughness = mix(roughness, TUNING_shore_earthRoughness, shore.y);
		albedo = mix(albedo, at_least(ground_params[PARAMS_shore].xyz, least), shore.x);
		roughness = mix(roughness, TUNING_shore_wetRoughness, shore.x);
	}

	vec4 paving = ground_roads(xy, footprint, paved, vec4(albedo, roughness));
	float bed = ss(-aa, max(aa, TUNING_shore_bedEaseM), water);
	albedo = mix(paving.xyz, ground_params[PARAMS_waterBed].xyz, bed);
	return vec4(max(albedo, vec3(0.0)), paving.w);
}

// The class view -----------------------------------------------------------------

// A distance byte over 255, `inside` metres inside an edge
// (`groundClasses.ts`: 8 steps a metre, the edge at 128).
float class_distance(float inside) {
	return clamp(roundEven(128.0 - inside * 8.0), 1.0, 255.0) / 255.0;
}

// The class view's three bytes over 255 (`groundClasses`): outside the
// paving, outside the water, then forest << 6 | kind << 2 | hash & 3.
vec3 ground_classes(vec2 xy, float footprint, vec4 site, float water) {
	uint forest = 0u;
	if (site.w >= 0.0) {
		forest = 2u;
	} else {
		vec2 floor_weight = ground_floor(xy, footprint, site.w);
		if (max(floor_weight.x, floor_weight.y) > 0.5) {
			forest = 1u;
		}
	}
	uint kind = min(uint(plot_field(int(site.x), PLOTS_detail).y), 15u);
	uint hash = pcg_hash(uint(site.x)) & 3u;
	uint packed = (forest << 6u) | (kind << 2u) | hash;
	return vec3(class_distance(site.z), class_distance(water), float(packed) / 255.0);
}
