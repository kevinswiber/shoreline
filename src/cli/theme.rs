//! Theme selection for `shore diff`'s truecolor lane: preference grammar,
//! precedence, palettes, and terminal-background detection.

// Consumed by the diff render path once the wiring lands; until then the
// items here are dead code to the binary. Remove this allow with that wiring.
#![allow(dead_code)]

/// Resolved lightness class for the truecolor palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DiffMode {
    Light,
    Dark,
}

/// A parsed theme preference: detect, force a mode's built-in palette, or a
/// named embedded theme. Parsing is infallible — unknown names are resolved
/// (and rejected or warned about) later, with source-aware posture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ThemePreference {
    Auto,
    Mode(DiffMode),
    Named(String),
}

/// The theme value grammar: `auto` (and any `auto:*` variant, for BAT_THEME
/// compatibility), `light`, `dark`, `default` (bat back-compat: the dark
/// default), else a verbatim theme name. Keywords are case-sensitive
/// lowercase; theme names are case-sensitive too, so anything unrecognized
/// passes through untouched.
pub(super) fn parse_theme_value(value: &str) -> ThemePreference {
    let value = value.trim();
    if value == "auto" || value.starts_with("auto:") {
        return ThemePreference::Auto;
    }
    match value {
        "light" => ThemePreference::Mode(DiffMode::Light),
        "dark" | "default" => ThemePreference::Mode(DiffMode::Dark),
        other => ThemePreference::Named(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keywords_and_names() {
        assert_eq!(parse_theme_value("auto"), ThemePreference::Auto);
        assert_eq!(
            parse_theme_value("light"),
            ThemePreference::Mode(DiffMode::Light)
        );
        assert_eq!(
            parse_theme_value("dark"),
            ThemePreference::Mode(DiffMode::Dark)
        );
        // bat back-compat: an explicit "default" always means the dark default.
        assert_eq!(
            parse_theme_value("default"),
            ThemePreference::Mode(DiffMode::Dark)
        );
        // bat's extended auto grammar collapses to Auto (shore's gate governs).
        assert_eq!(parse_theme_value("auto:always"), ThemePreference::Auto);
        assert_eq!(parse_theme_value("auto:system"), ThemePreference::Auto);
        // Anything else is a named theme, verbatim (names are case-sensitive).
        assert_eq!(
            parse_theme_value("Monokai Extended"),
            ThemePreference::Named("Monokai Extended".to_string())
        );
        // Keywords are case-sensitive lowercase; "Dark" is a (bogus) name, not a mode.
        assert_eq!(
            parse_theme_value("Dark"),
            ThemePreference::Named("Dark".to_string())
        );
    }

    #[test]
    fn trims_surrounding_whitespace_only() {
        assert_eq!(
            parse_theme_value("  light "),
            ThemePreference::Mode(DiffMode::Light)
        );
        // Interior whitespace stays (theme names contain spaces).
        assert_eq!(
            parse_theme_value(" Solarized (dark) "),
            ThemePreference::Named("Solarized (dark)".to_string())
        );
    }
}
