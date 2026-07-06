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

/// Where a theme selection came from — governs the unknown-name posture:
/// explicit sources fail hard, the inherited source warns and falls back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ThemeSource {
    Explicit,
    Inherited,
    Default,
}

/// A resolved preference plus its provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ThemeSelection {
    pub(super) preference: ThemePreference,
    pub(super) source: ThemeSource,
}

/// Pure precedence core: `--theme` flag > `SHORE_THEME` > `BAT_THEME` > Auto.
/// Blank (empty/whitespace) values are no selection. Injected values keep it
/// unit-testable without touching (or racing on) the process environment.
pub(super) fn resolve_theme_selection(
    flag: Option<&str>,
    shore_env: Option<&str>,
    bat_env: Option<&str>,
) -> ThemeSelection {
    fn pick(value: Option<&str>) -> Option<&str> {
        value.map(str::trim).filter(|value| !value.is_empty())
    }
    if let Some(value) = pick(flag).or(pick(shore_env)) {
        return ThemeSelection {
            preference: parse_theme_value(value),
            source: ThemeSource::Explicit,
        };
    }
    if let Some(value) = pick(bat_env) {
        return ThemeSelection {
            preference: parse_theme_value(value),
            source: ThemeSource::Inherited,
        };
    }
    ThemeSelection {
        preference: ThemePreference::Auto,
        source: ThemeSource::Default,
    }
}

/// Read `SHORE_THEME` / `BAT_THEME` and delegate to the pure core. The single
/// theme-env read site (the `SHORE_FORMAT` convention, `src/cli/output.rs`).
pub(super) fn theme_selection_from_env(flag: Option<&str>) -> ThemeSelection {
    let shore = std::env::var("SHORE_THEME").ok();
    let bat = std::env::var("BAT_THEME").ok();
    resolve_theme_selection(flag, shore.as_deref(), bat.as_deref())
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
    fn precedence_flag_over_shore_env_over_bat_env() {
        let sel = resolve_theme_selection(Some("light"), Some("dark"), Some("Nord"));
        assert_eq!(sel.preference, ThemePreference::Mode(DiffMode::Light));
        assert_eq!(sel.source, ThemeSource::Explicit);

        let sel = resolve_theme_selection(None, Some("dark"), Some("Nord"));
        assert_eq!(sel.preference, ThemePreference::Mode(DiffMode::Dark));
        assert_eq!(sel.source, ThemeSource::Explicit);

        let sel = resolve_theme_selection(None, None, Some("Nord"));
        assert_eq!(sel.preference, ThemePreference::Named("Nord".to_string()));
        assert_eq!(sel.source, ThemeSource::Inherited);

        let sel = resolve_theme_selection(None, None, None);
        assert_eq!(sel.preference, ThemePreference::Auto);
        assert_eq!(sel.source, ThemeSource::Default);
    }

    #[test]
    fn empty_or_blank_values_are_no_selection() {
        // Unset and empty env are the same thing (SHORE_FORMAT precedent).
        let sel = resolve_theme_selection(None, Some(""), Some("  "));
        assert_eq!(sel.preference, ThemePreference::Auto);
        assert_eq!(sel.source, ThemeSource::Default);
        // An empty SHORE_THEME does not mask BAT_THEME.
        let sel = resolve_theme_selection(None, Some(""), Some("Nord"));
        assert_eq!(sel.source, ThemeSource::Inherited);
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
