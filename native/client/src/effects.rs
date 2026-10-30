//! Filesystem transport of the authored effect sheets in presentation order.
use renderer_core::effects::EffectSheets;
use std::path::Path;
use std::sync::Arc;

/// Atlas layer order is shared with browser source transport and presentation.
const SHEETS: &str = include_str!("../../../assets/effect-sheets.json");

/// Read encoded sources; the shared decoder owns premultiplication and mips.
pub fn load_sheets(dir: &Path, side: u32) -> Result<Arc<EffectSheets>, String> {
    let files: Vec<String> = serde_json::from_str(SHEETS).map_err(|e| e.to_string())?;
    let sources = files
        .iter()
        .map(|file| {
            let path = dir.join(file);
            std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let refs = sources.iter().map(Vec::as_slice).collect::<Vec<_>>();
    EffectSheets::decode(side, &refs).map(Arc::new)
}
