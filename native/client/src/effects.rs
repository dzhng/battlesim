//! Filesystem transport of the authored effect sheets in presentation order.
use renderer_core::effects::EffectSheets;
use std::path::Path;
use std::sync::Arc;

/// The browser's flipbook list: its imports name each sheet, and its
/// `FLIPBOOKS` rows give the layer order.
const FLIPBOOKS_TS: &str =
    include_str!("../../../packages/battle-renderer/src/effects/flipbooks.ts");
/// The sheets' file names in layer order, read from `flipbooks.ts`.
fn sheet_files() -> Vec<String> {
    let imports: Vec<(&str, &str)> = FLIPBOOKS_TS
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("import ")?;
            let (name, from) = rest.split_once(" from ")?;
            let file = from.split("/effects/").nth(1)?.split('?').next()?;
            Some((name.trim(), file))
        })
        .collect();
    let files: Vec<String> = FLIPBOOKS_TS
        .split("{ url: ")
        .skip(1)
        .filter_map(|row| {
            let name = row.split(',').next()?.trim();
            imports
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, f)| f.to_string())
        })
        .collect();
    assert!(!files.is_empty(), "flipbooks.ts lists no sheet");
    files
}

/// Read encoded sources; the shared decoder owns premultiplication and mips.
pub fn load_sheets(dir: &Path, side: u32) -> Result<Arc<EffectSheets>, String> {
    let sources = sheet_files()
        .iter()
        .map(|file| {
            let path = dir.join(file);
            std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let refs = sources.iter().map(Vec::as_slice).collect::<Vec<_>>();
    EffectSheets::decode(side, &refs).map(Arc::new)
}
