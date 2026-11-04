use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::PathBuf,
};

fn insert_block(
    blocks: &mut BTreeMap<String, (PathBuf, String)>,
    id: String,
    body: &str,
    path: &std::path::Path,
) {
    if let Some((previous, _)) = blocks.get(&id) {
        panic!(
            "{}: duplicate WGSL block {id}, first defined in {}",
            path.display(),
            previous.display()
        );
    }
    blocks.insert(id, (path.to_owned(), body.trim_end().to_owned()));
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let modules = root.join("src/shaders/modules");
    let source_index = root.join("src/shaders/pipeline_sources.txt");
    println!("cargo:rerun-if-changed={}", modules.display());
    println!("cargo:rerun-if-changed={}", source_index.display());

    let mut blocks = BTreeMap::<String, (PathBuf, String)>::new();
    for entry in fs::read_dir(&modules).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|x| x.to_str()) != Some("wgsl") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let mut current = None;
        let mut body = String::new();
        for line in text.lines() {
            if let Some(id) = line.strip_prefix("// @block ") {
                if let Some(old) = current.replace(id.trim().to_owned()) {
                    insert_block(&mut blocks, old, &body, &path);
                    body.clear();
                }
            } else if current.is_some() {
                body.push_str(line);
                body.push('\n');
            }
        }
        if let Some(id) = current {
            insert_block(&mut blocks, id, &body, &path);
        }
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let source_text = fs::read_to_string(&source_index).unwrap();
    let mut shaders = BTreeSet::new();
    let entries: Vec<_> = source_text
        .lines()
        .enumerate()
        .map(|(line_number, line)| {
            let (shader, ids) = line.split_once('|').unwrap_or_else(|| {
                panic!(
                    "{}:{}: expected shader|blocks",
                    source_index.display(),
                    line_number + 1
                )
            });
            assert!(
                shaders.insert(shader),
                "{}:{}: duplicate shader {shader}",
                source_index.display(),
                line_number + 1
            );
            (shader, ids)
        })
        .collect();
    let mut generated = format!(
        "pub const BUILTIN_SHADERS: &[&str] = &{:?};\n\n",
        entries.iter().map(|(shader, _)| shader).collect::<Vec<_>>()
    );
    generated.push_str("fn assembled_source(name: &str) -> Option<&'static str> { match name {\n");
    let mut referenced = BTreeSet::new();
    for (shader, ids) in entries {
        let mut source = String::new();
        for id in ids.split(',') {
            let (id, group) = id.split_once('@').map_or((id, None), |(id, group)| {
                let group = group
                    .parse::<u32>()
                    .unwrap_or_else(|_| panic!("{shader}: bad WGSL group {group} for {id}"));
                (id, Some(group))
            });
            let (_, body) = blocks
                .get(id)
                .unwrap_or_else(|| panic!("{shader}: missing WGSL block {id}"));
            referenced.insert(id);
            let context = format!("{shader}: block {id}");
            let has_group = body
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .any(|token| token == "GROUP");
            assert!(
                !has_group || group.is_some(),
                "{context} contains GROUP and requires @N"
            );
            assert!(
                has_group || group.is_none(),
                "{context} has no GROUP but was given @N"
            );
            let mut body = body.clone();
            if let Some(group) = group {
                body = body.replace("@group(GROUP)", &format!("@group({group})"));
            }
            source.push_str(&body);
            source.push_str("\n\n");
        }
        let path = out_dir.join(format!("assembled-{shader}"));
        fs::write(&path, source).unwrap();
        generated.push_str(&format!("{shader:?} => Some(include_str!({:?})),\n", path));
    }
    for id in blocks.keys() {
        assert!(
            referenced.contains(id.as_str()),
            "unreferenced WGSL block {id}"
        );
    }
    generated.push_str("_ => None, } }\n");
    fs::write(out_dir.join("pipeline_export.rs"), generated).unwrap();
}
