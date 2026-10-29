use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn replace_identifiers(source: &str, aliases: &[(String, String)]) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    let mut copied = 0;
    while at < bytes.len() {
        if !(bytes[at].is_ascii_alphabetic() || bytes[at] == b'_') {
            at += 1;
            continue;
        }
        let start = at;
        at += 1;
        while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_') {
            at += 1;
        }
        let token = &source[start..at];
        out.push_str(&source[copied..start]);
        if let Some((_, replacement)) = aliases.iter().find(|(name, _)| name == token) {
            out.push_str(replacement);
        } else {
            out.push_str(token);
        }
        copied = at;
    }
    out.push_str(&source[copied..]);
    out
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let modules = root.join("src/shaders/modules");
    let source_index = root.join("src/shaders/pipeline_sources.txt");
    println!("cargo:rerun-if-changed={}", modules.display());
    println!("cargo:rerun-if-changed={}", source_index.display());

    let mut blocks = BTreeMap::<String, String>::new();
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
                    blocks.insert(old, body.trim_end().to_owned());
                    body.clear();
                }
            } else if current.is_some() {
                body.push_str(line);
                body.push('\n');
            }
        }
        if let Some(id) = current {
            blocks.insert(id, body.trim_end().to_owned());
        }
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let source_text = fs::read_to_string(&source_index).unwrap();
    let pipeline_count = source_text.lines().count();
    let mut generated = format!("pub const BUILTIN_PIPELINE_COUNT: usize = {pipeline_count};\n\n");
    let shaders: Vec<String> = source_text
        .lines()
        .filter_map(|line| line.split_once('|').map(|(shader, _)| shader.to_owned()))
        .collect();
    generated.push_str(&format!(
        "pub const BUILTIN_SHADERS: &[&str] = &{:?};\n\n",
        shaders
    ));
    generated.push_str("fn assembled_source(name: &str) -> Option<&'static str> { match name {\n");
    for line in source_text.lines() {
        let Some((shader, ids)) = line.split_once('|') else {
            continue;
        };
        let mut source = String::new();
        for reference in ids.split(',') {
            let (id, aliases) = reference
                .split_once('[')
                .map_or((reference, ""), |(id, rest)| {
                    (id, rest.strip_suffix(']').unwrap_or(rest))
                });
            let aliases: Vec<_> = aliases
                .split(';')
                .filter(|alias| !alias.is_empty())
                .map(|alias| {
                    let (from, to) = alias
                        .split_once('=')
                        .unwrap_or_else(|| panic!("bad WGSL alias {alias}"));
                    (from.to_owned(), to.to_owned())
                })
                .collect();
            let body = blocks
                .get(id)
                .unwrap_or_else(|| panic!("missing WGSL block {id}"));
            source.push_str(&replace_identifiers(body, &aliases));
            source.push_str("\n\n");
        }
        let path = out_dir.join(format!("assembled-{shader}"));
        fs::write(&path, source).unwrap();
        generated.push_str(&format!("{shader:?} => Some(include_str!({:?})),\n", path));
    }
    generated.push_str("_ => None, } }\n");
    fs::write(out_dir.join("pipeline_export.rs"), generated).unwrap();
}
