//! What the player reads, kept apart from what the code addresses. A catalog
//! entry's id (`eastern_t_72_t_72b3_2016`, `main_gun`, `ground_tank_ap`) is
//! never shown; its label (`T-72B3`, `Main Gun`, `AP Shell`) is authored
//! beside it, and its icon names a drawing the asset tool generates.
//!
//! A label is concise when it reads as a name on a card, not an identifier
//! or a sentence:
//! - at most [`LABEL_MAX_CHARS`] characters, single-spaced, no space at
//!   either end;
//! - it opens with a capital letter or a digit (`rifles` is a slug);
//! - letters, digits, spaces and `- / . ×` only: no underscore, bracket,
//!   comma or other punctuation that turns a name into a description;
//! - no word written twice, words split at spaces, hyphens and slashes (so
//!   `BMP IFV Family BMP-3`, `Tigr Tigr-M` and `T-72 T-72B3` are refused:
//!   the family repeated before its variant).

/// The longest label a card shows whole: "M1A2 SEP v3 Trophy".
pub const LABEL_MAX_CHARS: usize = 18;

/// Why `label` is not a concise player-facing name, if it isn't.
pub fn check_label(label: &str) -> Result<(), String> {
    let refuse = |why: &str| Err(format!("name {label:?} {why}"));
    let chars = label.chars().count();
    if chars == 0 {
        return refuse("is empty");
    }
    if chars > LABEL_MAX_CHARS {
        return refuse(&format!("is longer than {LABEL_MAX_CHARS} characters"));
    }
    if label.trim() != label || label.contains("  ") {
        return refuse("needs single spaces between words and none at either end");
    }
    if !label
        .chars()
        .next()
        .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit())
    {
        return refuse("must open with a capital letter or a digit");
    }
    if let Some(c) = label
        .chars()
        .find(|&c| !(c.is_alphanumeric() || matches!(c, ' ' | '-' | '/' | '.' | '×')))
    {
        return refuse(&format!("holds {c:?}: a label is words, not an identifier"));
    }
    let words: Vec<String> = label
        .split([' ', '-', '/'])
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    for (i, w) in words.iter().enumerate() {
        if words[..i].contains(w) {
            return refuse(&format!("repeats {w:?}"));
        }
    }
    Ok(())
}

/// Why `icon` cannot name a generated icon file, if it can't: a lowercase
/// id (`ap_shell`), drawn under `assets/icons/weapons/<icon>.svg`.
pub fn check_icon(icon: &str) -> Result<(), String> {
    let id = icon.starts_with(|c: char| c.is_ascii_lowercase())
        && icon
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if id {
        Ok(())
    } else {
        Err(format!(
            "icon {icon:?} must be a lowercase icon id (letters, digits, underscores)"
        ))
    }
}
