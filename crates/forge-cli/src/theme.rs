//! TTY-aware ANSI theme, NO_COLOR compliance, and visual score meter for `eforge`.
//!
//! Follows standard terminal conventions:
//! - [`NO_COLOR`](https://no-color.org/): disables ANSI colors when present and non-empty.
//! - [`CLICOLOR_FORCE`](https://bixense.com/clicolors/): forces ANSI colors when set and != "0".
//! - [`CLICOLOR`](https://bixense.com/clicolors/): disables colors when set to "0".
//! - `std::io::IsTerminal`: detects whether stdout is an interactive terminal.

use std::io::IsTerminal;

use forge_core::Severity;

/// Color configuration mode for terminal output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Automatically detects TTY capability and environment variables (`NO_COLOR`, `CLICOLOR_FORCE`).
    #[default]
    Auto,
    /// Always emits ANSI escape sequences and Unicode glyphs.
    Always,
    /// Never emits ANSI escape sequences or non-ASCII characters.
    Never,
}

/// Evaluates whether ANSI escape colors should be enabled for a given configuration and environment.
pub fn eval_color_supported(
    mode: ColorMode,
    is_tty: bool,
    no_color: Option<&str>,
    clicolor_force: Option<&str>,
    clicolor: Option<&str>,
) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            // 1. CLICOLOR_FORCE != 0 forces colors even in non-TTY environments.
            if let Some(force) = clicolor_force {
                if !force.is_empty() && force != "0" {
                    return true;
                }
            }

            // 2. NO_COLOR disables colors when set and non-empty.
            if let Some(nc) = no_color {
                if !nc.is_empty() {
                    return false;
                }
            }

            // 3. CLICOLOR=0 disables colors.
            if let Some(cli) = clicolor {
                if cli == "0" {
                    return false;
                }
            }

            // 4. Default to interactive TTY check on stdout.
            is_tty
        }
    }
}

/// Evaluates whether Unicode graphical glyphs (e.g. `█`, `░`, `✓`, `✗`) are supported.
pub fn eval_unicode_supported(mode: ColorMode, is_tty: bool, no_color: Option<&str>) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            if let Some(nc) = no_color {
                if !nc.is_empty() {
                    return false;
                }
            }
            is_tty
        }
    }
}

/// Evaluates whether ANSI escape colors should be enabled based on [`ColorMode`] and active environment.
pub fn is_color_supported(mode: ColorMode) -> bool {
    let no_color = std::env::var("NO_COLOR").ok();
    let clicolor_force = std::env::var("CLICOLOR_FORCE").ok();
    let clicolor = std::env::var("CLICOLOR").ok();
    let is_tty = std::io::stdout().is_terminal();

    eval_color_supported(
        mode,
        is_tty,
        no_color.as_deref(),
        clicolor_force.as_deref(),
        clicolor.as_deref(),
    )
}

/// Evaluates whether Unicode graphical glyphs (e.g. `█`, `░`, `✓`, `✗`) are supported based on active environment.
pub fn is_unicode_supported(mode: ColorMode) -> bool {
    let no_color = std::env::var("NO_COLOR").ok();
    let is_tty = std::io::stdout().is_terminal();

    eval_unicode_supported(mode, is_tty, no_color.as_deref())
}

/// Active terminal styling theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub color_enabled: bool,
    pub unicode_enabled: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::detect(ColorMode::Auto)
    }
}

impl Theme {
    /// Detects terminal capabilities from environment and TTY state using the specified [`ColorMode`].
    pub fn detect(mode: ColorMode) -> Self {
        Self {
            color_enabled: is_color_supported(mode),
            unicode_enabled: is_unicode_supported(mode),
        }
    }

    /// Creates a plain theme without ANSI colors or Unicode glyphs.
    pub const fn plain() -> Self {
        Self {
            color_enabled: false,
            unicode_enabled: false,
        }
    }

    /// Creates a rich theme with ANSI colors and Unicode glyphs enabled.
    pub const fn colored() -> Self {
        Self {
            color_enabled: true,
            unicode_enabled: true,
        }
    }

    /// Formats `text` with bold styling if colors are enabled.
    pub fn bold(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[1m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` with dim styling if colors are enabled.
    pub fn dim(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[2m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` in cyan.
    pub fn cyan(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[36m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` in green.
    pub fn green(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[32m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` in yellow.
    pub fn yellow(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[33m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` in red.
    pub fn red(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[31m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` in magenta.
    pub fn magenta(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[35m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` in white.
    pub fn white(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[37m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` in blue.
    pub fn blue(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[34m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Formats `text` in gray.
    pub fn gray(&self, text: &str) -> String {
        if self.color_enabled {
            format!("\x1b[90m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Severity badge for `CRITICAL` findings (bold red).
    pub fn badge_critical(&self) -> String {
        if self.color_enabled {
            "\x1b[1;31m[CRITICAL]\x1b[0m".to_string()
        } else {
            "[CRITICAL]".to_string()
        }
    }

    /// Severity badge for `HIGH` findings (red).
    pub fn badge_high(&self) -> String {
        if self.color_enabled {
            "\x1b[31m[HIGH]\x1b[0m".to_string()
        } else {
            "[HIGH]".to_string()
        }
    }

    /// Severity badge for `MEDIUM` findings (yellow).
    pub fn badge_medium(&self) -> String {
        if self.color_enabled {
            "\x1b[33m[MEDIUM]\x1b[0m".to_string()
        } else {
            "[MEDIUM]".to_string()
        }
    }

    /// Severity badge for `LOW` findings (blue).
    pub fn badge_low(&self) -> String {
        if self.color_enabled {
            "\x1b[34m[LOW]\x1b[0m".to_string()
        } else {
            "[LOW]".to_string()
        }
    }

    /// Severity badge for `INFO` findings (cyan).
    pub fn badge_info(&self) -> String {
        if self.color_enabled {
            "\x1b[36m[INFO]\x1b[0m".to_string()
        } else {
            "[INFO]".to_string()
        }
    }

    /// Renders a formatted severity badge for any [`Severity`].
    pub fn badge_severity(&self, severity: Severity) -> String {
        match severity {
            Severity::Critical => self.badge_critical(),
            Severity::High => self.badge_high(),
            Severity::Medium => self.badge_medium(),
            Severity::Low => self.badge_low(),
            Severity::Info => self.badge_info(),
        }
    }

    /// Checkmark / Success marker (`✓` in green or `[OK]`).
    pub fn status_ok(&self, text: &str) -> String {
        if self.unicode_enabled {
            if self.color_enabled {
                format!("\x1b[32m✓\x1b[0m {text}")
            } else {
                format!("✓ {text}")
            }
        } else if self.color_enabled {
            format!("\x1b[32m[OK]\x1b[0m {text}")
        } else {
            format!("[OK] {text}")
        }
    }

    /// Failure marker (`✗` in red or `[FAIL]`).
    pub fn status_fail(&self, text: &str) -> String {
        if self.unicode_enabled {
            if self.color_enabled {
                format!("\x1b[31m✗\x1b[0m {text}")
            } else {
                format!("✗ {text}")
            }
        } else if self.color_enabled {
            format!("\x1b[31m[FAIL]\x1b[0m {text}")
        } else {
            format!("[FAIL] {text}")
        }
    }

    /// Warning marker (`⚠` in yellow or `[WARN]`).
    pub fn status_warn(&self, text: &str) -> String {
        if self.unicode_enabled {
            if self.color_enabled {
                format!("\x1b[33m⚠\x1b[0m {text}")
            } else {
                format!("⚠ {text}")
            }
        } else if self.color_enabled {
            format!("\x1b[33m[WARN]\x1b[0m {text}")
        } else {
            format!("[WARN] {text}")
        }
    }

    /// Renders the visual score meter using this theme's configuration.
    pub fn render_score_meter(&self, score: u8) -> String {
        render_score_meter_internal(score, self.color_enabled, self.unicode_enabled)
    }
}

/// Translates a 0..=100 health score into a human-readable quality rating.
pub fn health_rating(score: u8) -> &'static str {
    match score {
        85..=100 => "Good",
        70..=84 => "Fair",
        50..=69 => "Needs Attention",
        _ => "Poor",
    }
}

/// Internal helper to render the visual score meter.
fn render_score_meter_internal(score: u8, color_enabled: bool, unicode_enabled: bool) -> String {
    let s = score.min(100);
    let filled = (s as usize * 10) / 100;
    let empty = 10 - filled;

    let (fill_str, empty_str) = if unicode_enabled {
        ("█".repeat(filled), "░".repeat(empty))
    } else {
        ("=".repeat(filled), ".".repeat(empty))
    };

    let bar = format!("{fill_str}{empty_str}");
    let rating = health_rating(s);

    if color_enabled {
        let color_code = match s {
            85..=100 => "\x1b[32m", // Green
            70..=84 => "\x1b[33m",  // Yellow
            50..=69 => "\x1b[33m",  // Yellow
            _ => "\x1b[31m",        // Red
        };
        format!("{color_code}[{bar}] {s}/100 ({rating})\x1b[0m")
    } else {
        format!("[{bar}] {s}/100 ({rating})")
    }
}

/// Renders a 10-segment visual health score bar meter respecting [`ColorMode`].
///
/// In TTY/Unicode mode:
/// `[█████████░] 92/100 (Good)`
///
/// In plain/non-TTY fallback mode:
/// `[=========.] 92/100 (Good)`
pub fn render_score_meter(score: u8, color_mode: ColorMode) -> String {
    let theme = Theme::detect(color_mode);
    render_score_meter_internal(score, theme.color_enabled, theme.unicode_enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_color_environment_no_color() {
        // NO_COLOR set to any non-empty value disables colors even if stdout is a TTY.
        assert!(!eval_color_supported(
            ColorMode::Auto,
            true,
            Some("1"),
            None,
            None
        ));
        assert!(!eval_color_supported(
            ColorMode::Auto,
            true,
            Some("true"),
            None,
            None
        ));

        // NO_COLOR empty does not disable colors.
        assert!(eval_color_supported(
            ColorMode::Auto,
            true,
            Some(""),
            None,
            None
        ));
    }

    #[test]
    fn test_eval_color_clicolor_force() {
        // CLICOLOR_FORCE forces colors even in non-TTY and even if NO_COLOR is present.
        assert!(eval_color_supported(
            ColorMode::Auto,
            false,
            Some("1"),
            Some("1"),
            None
        ));
        assert!(eval_color_supported(
            ColorMode::Auto,
            false,
            None,
            Some("true"),
            None
        ));

        // CLICOLOR_FORCE="0" does not force colors.
        assert!(!eval_color_supported(
            ColorMode::Auto,
            false,
            None,
            Some("0"),
            None
        ));
    }

    #[test]
    fn test_eval_color_clicolor_zero() {
        // CLICOLOR="0" disables colors on TTY.
        assert!(!eval_color_supported(
            ColorMode::Auto,
            true,
            None,
            None,
            Some("0")
        ));
    }

    #[test]
    fn test_eval_unicode_supported() {
        assert!(eval_unicode_supported(ColorMode::Always, false, None));
        assert!(!eval_unicode_supported(ColorMode::Never, true, None));
        assert!(eval_unicode_supported(ColorMode::Auto, true, None));
        assert!(!eval_unicode_supported(ColorMode::Auto, false, None));
        assert!(!eval_unicode_supported(ColorMode::Auto, true, Some("1")));
    }

    #[test]
    fn test_health_rating_categories() {
        assert_eq!(health_rating(100), "Good");
        assert_eq!(health_rating(92), "Good");
        assert_eq!(health_rating(85), "Good");
        assert_eq!(health_rating(84), "Fair");
        assert_eq!(health_rating(70), "Fair");
        assert_eq!(health_rating(69), "Needs Attention");
        assert_eq!(health_rating(50), "Needs Attention");
        assert_eq!(health_rating(49), "Poor");
        assert_eq!(health_rating(0), "Poor");
    }

    #[test]
    fn test_score_meter_plain_boundary_scores() {
        // Plain (no color, ASCII chars)
        let meter_100 = render_score_meter_internal(100, false, false);
        assert_eq!(meter_100, "[==========] 100/100 (Good)");

        let meter_92 = render_score_meter_internal(92, false, false);
        assert_eq!(meter_92, "[=========.] 92/100 (Good)");

        let meter_50 = render_score_meter_internal(50, false, false);
        assert_eq!(meter_50, "[=====.....] 50/100 (Needs Attention)");

        let meter_0 = render_score_meter_internal(0, false, false);
        assert_eq!(meter_0, "[..........] 0/100 (Poor)");
    }

    #[test]
    fn test_score_meter_unicode_boundary_scores() {
        // Unicode (no color, full blocks and shade blocks)
        let meter_100 = render_score_meter_internal(100, false, true);
        assert_eq!(meter_100, "[██████████] 100/100 (Good)");

        let meter_92 = render_score_meter_internal(92, false, true);
        assert_eq!(meter_92, "[█████████░] 92/100 (Good)");

        let meter_50 = render_score_meter_internal(50, false, true);
        assert_eq!(meter_50, "[█████░░░░░] 50/100 (Needs Attention)");

        let meter_0 = render_score_meter_internal(0, false, true);
        assert_eq!(meter_0, "[░░░░░░░░░░] 0/100 (Poor)");
    }

    #[test]
    fn test_score_meter_colored_contains_ansi_escapes() {
        let meter_92 = render_score_meter_internal(92, true, true);
        assert!(meter_92.contains("\x1b[32m")); // Green
        assert!(meter_92.contains("[█████████░] 92/100 (Good)"));
        assert!(meter_92.ends_with("\x1b[0m"));

        let meter_0 = render_score_meter_internal(0, true, true);
        assert!(meter_0.contains("\x1b[31m")); // Red
        assert!(meter_0.contains("[░░░░░░░░░░] 0/100 (Poor)"));
    }

    #[test]
    fn test_theme_styling_primitives_plain() {
        let theme = Theme::plain();
        assert_eq!(theme.bold("hello"), "hello");
        assert_eq!(theme.dim("world"), "world");
        assert_eq!(theme.red("alert"), "alert");
        assert_eq!(theme.green("ok"), "ok");
        assert_eq!(theme.yellow("warn"), "warn");
        assert_eq!(theme.cyan("info"), "info");
        assert_eq!(theme.magenta("mag"), "mag");
        assert_eq!(theme.white("white"), "white");
        assert_eq!(theme.blue("blue"), "blue");
        assert_eq!(theme.gray("gray"), "gray");
    }

    #[test]
    fn test_theme_styling_primitives_colored() {
        let theme = Theme::colored();
        assert_eq!(theme.bold("hello"), "\x1b[1mhello\x1b[0m");
        assert_eq!(theme.dim("world"), "\x1b[2mworld\x1b[0m");
        assert_eq!(theme.red("alert"), "\x1b[31malert\x1b[0m");
        assert_eq!(theme.green("ok"), "\x1b[32mok\x1b[0m");
        assert_eq!(theme.yellow("warn"), "\x1b[33mwarn\x1b[0m");
        assert_eq!(theme.cyan("info"), "\x1b[36minfo\x1b[0m");
        assert_eq!(theme.magenta("mag"), "\x1b[35mmag\x1b[0m");
        assert_eq!(theme.white("white"), "\x1b[37mwhite\x1b[0m");
        assert_eq!(theme.blue("blue"), "\x1b[34mblue\x1b[0m");
        assert_eq!(theme.gray("gray"), "\x1b[90mgray\x1b[0m");
    }

    #[test]
    fn test_severity_badges_plain_and_colored() {
        let plain = Theme::plain();
        assert_eq!(plain.badge_critical(), "[CRITICAL]");
        assert_eq!(plain.badge_high(), "[HIGH]");
        assert_eq!(plain.badge_medium(), "[MEDIUM]");
        assert_eq!(plain.badge_low(), "[LOW]");
        assert_eq!(plain.badge_info(), "[INFO]");

        let colored = Theme::colored();
        assert_eq!(colored.badge_critical(), "\x1b[1;31m[CRITICAL]\x1b[0m");
        assert_eq!(colored.badge_high(), "\x1b[31m[HIGH]\x1b[0m");
        assert_eq!(colored.badge_medium(), "\x1b[33m[MEDIUM]\x1b[0m");
        assert_eq!(colored.badge_low(), "\x1b[34m[LOW]\x1b[0m");
        assert_eq!(colored.badge_info(), "\x1b[36m[INFO]\x1b[0m");

        // badge_severity
        assert_eq!(plain.badge_severity(Severity::High), "[HIGH]");
        assert_eq!(
            colored.badge_severity(Severity::Critical),
            "\x1b[1;31m[CRITICAL]\x1b[0m"
        );
    }

    #[test]
    fn test_status_markers() {
        let plain = Theme::plain();
        assert_eq!(plain.status_ok("built"), "[OK] built");
        assert_eq!(plain.status_fail("error"), "[FAIL] error");
        assert_eq!(plain.status_warn("caution"), "[WARN] caution");

        let colored = Theme::colored();
        assert_eq!(colored.status_ok("built"), "\x1b[32m✓\x1b[0m built");
        assert_eq!(colored.status_fail("error"), "\x1b[31m✗\x1b[0m error");
        assert_eq!(colored.status_warn("caution"), "\x1b[33m⚠\x1b[0m caution");
    }

    #[test]
    fn test_color_mode_explicit() {
        assert!(is_color_supported(ColorMode::Always));
        assert!(!is_color_supported(ColorMode::Never));
        assert!(is_unicode_supported(ColorMode::Always));
        assert!(!is_unicode_supported(ColorMode::Never));
    }
}
