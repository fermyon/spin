//! Human-readable descriptions for directory relationships,
//! and helpers for standard display.

use std::path::Path;

fn parent_rel(distance: usize) -> String {
    match distance {
        0 => "".to_owned(),
        1 => "parent".to_owned(),
        2 => "grandparent".to_owned(),
        _ => format!("{}grandparent", "great-".repeat(distance - 2)),
    }
}

pub fn notify_if_nondefault_rel(manifest_file: &Path, distance: usize) {
    if distance > 0 {
        terminal::einfo!(
            "No 'spin.toml' in current directory.",
            "Using 'spin.toml' from {} directory ({})",
            parent_rel(distance),
            manifest_file.display(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ancestry_text_is_correct() {
        assert_eq!("parent", parent_rel(1));
        assert_eq!("grandparent", parent_rel(2));
        assert_eq!("great-grandparent", parent_rel(3));
        assert_eq!("great-great-great-grandparent", parent_rel(5)); // I hope you're happy Lann
    }
}
