fn main() {
    let mut failed = false;
    for path in std::env::args().skip(1) {
        let input = std::fs::read_to_string(&path).unwrap();
        let map: contract::map::MapDefinition = serde_json::from_str(&input).unwrap();
        match map.authored_props() {
            Ok(props) => println!("{path}: admitted {} physical props", props.len()),
            Err(error) => {println!("{path}: rejected {error}"); failed = true;}
        }
    }
    if failed {std::process::exit(1);}
}
