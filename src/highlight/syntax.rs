//! Language detection: map a diff file's paths to a syntect syntax, by extension only.

use std::sync::OnceLock;

use syntect::parsing::{SyntaxReference, SyntaxSet};

/// The stock syntect syntax set, loaded once. Uses the `_newlines` variant so the line tokenizer
/// can feed each line with a trailing `\n` (so end-of-line patterns match).
pub(crate) fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Detect a syntax from the diff paths (new then old), by extension only.
///
/// In-memory: never touches disk (we deliberately avoid `find_syntax_for_file`, which reads the
/// first line off disk). Unknown, absent, or stock-unsupported extensions (e.g. `.ts`) return
/// `None`, which the caller renders plain.
pub(crate) fn syntax_for_paths(
    new_path: Option<&str>,
    old_path: Option<&str>,
) -> Option<&'static SyntaxReference> {
    let ss = syntax_set();
    for path in [new_path, old_path].into_iter().flatten() {
        let p = std::path::Path::new(path);
        if let Some(ext) = p.extension().and_then(|e| e.to_str())
            && let Some(syntax) = ss.find_syntax_by_extension(ext)
        {
            return Some(syntax);
        }
        // Whole-name fallback for extensionless, name-keyed syntaxes (Makefile, Dockerfile, ...).
        if let Some(name) = p.file_name().and_then(|n| n.to_str())
            && let Some(syntax) = ss.find_syntax_by_extension(name)
        {
            return Some(syntax);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_known_extension() {
        assert!(syntax_for_paths(Some("src/main.rs"), None).is_some());
        assert!(syntax_for_paths(Some("a.py"), None).is_some());
    }

    #[test]
    fn unknown_or_missing_extension_is_none() {
        assert!(syntax_for_paths(Some("a.xyzzy"), None).is_none());
        assert!(syntax_for_paths(None, None).is_none());
        // TypeScript is NOT in the stock bundle (decided: stock-first) -> None -> plain.
        assert!(syntax_for_paths(Some("app.ts"), None).is_none());
    }

    #[test]
    fn prefers_new_path_then_old_path() {
        assert!(syntax_for_paths(None, Some("old.rs")).is_some());
    }
}
