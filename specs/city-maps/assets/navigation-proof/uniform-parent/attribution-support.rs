
#[derive(Default)]
struct Attribution {h:u64,unique_h:u64,get:u64,hit:u64,insert:u64,seen:Vec<u64>}
thread_local! {static ATTR:std::cell::RefCell<Attribution>=Default::default();}
fn attr_get(){ATTR.with(|a|a.borrow_mut().get+=1);}
fn attr_hit(){ATTR.with(|a|a.borrow_mut().hit+=1);}
fn attr_insert(){ATTR.with(|a|a.borrow_mut().insert+=1);}
fn attr_h(k:usize){ATTR.with(|a|{let mut a=a.borrow_mut();a.h+=1;
    let mask=1u64<<(k%64);if a.seen[k/64]&mask==0 {a.unique_h+=1;a.seen[k/64]|=mask;}});}
pub fn attribution_reset(cells:usize){ATTR.with(|a|*a.borrow_mut()=Attribution {
    seen:vec![0;cells.div_ceil(64)],..Default::default()});}
pub fn attribution()->serde_json::Value{ATTR.with(|a|{let a=a.borrow();serde_json::json!({
    "h_evaluations":a.h,"h_distinct_cells":a.unique_h,"scratch_tile_gets":a.get,
    "scratch_hits":a.hit,"scratch_entry_lookups":a.insert,"counter_seen_bytes":a.seen.capacity()*8,
    "timings":"instrumented counters, not production timing"})})}
