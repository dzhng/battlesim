pub use sim::{math,world};
#[allow(dead_code)]mod candidate;
#[allow(dead_code)]mod road_graph;
use contract::{command::RoutePolicy,map::{MapDefinition,MoverClass},scenario::{Rules,PushClass}};
use candidate::{Mobility,NavGrid};use math::v2;use world::WorldGeometry;
fn physical_road_tracer(){
 let rules:Rules=serde_json::from_value(sim::fixtures::village()).unwrap();
 let input=serde_json::json!({"size":[256,256],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"water":[{"rect":[118,0,20,256],"bed_z":-2,"surface_z":-0.5}],"bridges":[{"deck":"bridge_deck","center":[128,51],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}],"roads":[{"points":[[8,51],[248,51]],"width_m":8},{"points":[[32,8],[32,248]],"width_m":8},{"points":[[224,8],[224,248]],"width_m":8}]});
 let map:MapDefinition=serde_json::from_value(input).unwrap();let w=WorldGeometry::new(&map,&rules);let mut grid=NavGrid::build(&w,w.props().cloned(),0.3);
 let roads=[road_graph::Segment{a:road_graph::Point(8.0,51.0),b:road_graph::Point(248.0,51.0)},road_graph::Segment{a:road_graph::Point(32.0,8.0),b:road_graph::Point(32.0,248.0)},road_graph::Segment{a:road_graph::Point(224.0,8.0),b:road_graph::Point(224.0,248.0)}];
 for class in [MoverClass::Infantry,MoverClass::Vehicle]{let m=Mobility{off_road_mps:6.0,road_mps:12.0,forest_multiplier:0.4,half_width_m:if class==MoverClass::Infantry{0.5}else{1.8},class,push:PushClass::Heavy,drive:None};
 let a=road_graph::Point(13.0,90.0);let b=road_graph::Point(243.0,166.0);let r=road_graph::plan(&roads,a,b,256.0,|a,b,_|grid.prototype_segment_cost(v2(a.0,a.1),v2(b.0,b.1),&m,RoutePolicy::Fastest)).expect("accessible road crossing");let points:Vec<_>=r.points.iter().skip(1).map(|p|v2(p.0,p.1)).collect();assert!(grid.route_fits(v2(a.0,a.1),&points,&m));println!("{}",serde_json::json!({"physical_road_tracer":true,"class":format!("{class:?}"),"route":format!("{:?}",r.points),"cost":r.cost,"nodes":r.nodes,"relaxed":r.relaxed}));
 }
}
fn main(){physical_road_tracer();
 let side:f64=std::env::args().nth(1).unwrap_or("256".into()).parse().unwrap();
 let rules:Rules=serde_json::from_value(sim::fixtures::village()).unwrap();
 for two in [false,true] {
 let mut raw=serde_json::json!({"size":[side,side],"fog_cell_m":8,"height_grid_m":4,"slope_cutoff_deg":35,"water":[{"rect":[side/2.0-10.0,0,20,side],"bed_z":-2,"surface_z":-0.5}],"bridges":[{"deck":"bridge_deck","center":[side/2.0,side*0.2],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}]});
 if two {raw["bridges"].as_array_mut().unwrap().push(serde_json::json!({"deck":"bridge_deck","center":[side/2.0,side*0.8],"half_extents":[16,5],"yaw":0,"deck_z":0.1,"thickness_m":0.8}));}
 let map:MapDefinition=serde_json::from_value(raw.clone()).unwrap();let w=WorldGeometry::new(&map,&rules);let mut grid=NavGrid::build(&w,w.props().cloned(),0.3);
 for class in [MoverClass::Infantry,MoverClass::Vehicle] {for policy in [RoutePolicy::Shortest,RoutePolicy::Fastest] {for reverse in [false,true] {
 let m=Mobility{off_road_mps:6.0,road_mps:12.0,forest_multiplier:0.4,half_width_m:if class==MoverClass::Infantry {0.5}else{1.8},class,push:PushClass::Heavy,drive:None};
 let (a,b)=(v2(side*0.05,side*0.35),v2(side*0.95,side*0.65));let (a,b)=if reverse {(b,a)}else{(a,b)};
 let start=std::time::Instant::now();let (route,cost,samples)=grid.corridor_prototype(a,b,&m,policy).expect("bridge corridor");let ms=start.elapsed().as_secs_f64()*1000.0;
 assert!(grid.route_fits(a,&route,&m));assert_eq!(route.last(),Some(&b));assert_eq!(grid.corridor_prototype(a,b,&m,policy).unwrap().0,route);
 let old=grid.plan(a,b,&m,policy);let old_cost=if let candidate::Plan::Route(ref old_route)=old {let mut from=a;let mut c=0.0;for &to in old_route {c+=grid.prototype_segment_cost(from,to,&m,policy).unwrap();from=to;}Some(c)}else{None};
 let length=std::iter::once(a).chain(route.iter().copied()).collect::<Vec<_>>().windows(2).map(|p|(p[1]-p[0]).length()).sum::<f64>();
 println!("{}",serde_json::json!({"extent_m":side,"two_bridges":two,"class":format!("{class:?}"),"policy":format!("{policy:?}"),"reverse":reverse,"route":format!("{route:?}"),"policy_cost":cost,"length_m":length,"refinement_samples":samples,"candidate_ms":ms,"comparison_plan":format!("{old:?}"),"comparison_policy_cost":old_cost,"grid":grid.storage()}));
 }}}
 }
}
