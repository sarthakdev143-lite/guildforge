//! Role declarations and colors. See
//! [`docs/SCHEMA.md` §3.2](../../docs/SCHEMA.md).

use serde::{Deserialize, Serialize};

/// A role declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    /// Role name, 1-100 chars, unique within guild (case-insensitive).
    pub name: String,

    /// Role color (named, hex, rgb, or default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,

    /// Whether the role is hoisted (shown separately in the member list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hoist: Option<bool>,

    /// Whether the role can be mentioned by users with @rolename.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<bool>,

    /// List of permission names granted by this role. See
    /// [`docs/SCHEMA.md` §10](../../docs/SCHEMA.md).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,

    /// Explicit position override. By default roles are ordered by
    /// declaration order; `position` overrides this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,

    /// Path to a PNG icon (requires Discord Nitro boost level 2+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Unicode emoji to display as the role icon (alternative to `icon`).
    /// Custom-emoji icons are not supported in v1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<String>,
}

/// Color representation. See
/// [`docs/SCHEMA.md` §3.2](../../docs/SCHEMA.md).
///
/// Deserializes from a string. The format is auto-detected:
/// - `default` → [`Color::Default`]
/// - `red`, `blue`, etc. → [`Color::Named`] (see [`NamedColor`])
/// - `#RRGGBB` or `0xRRGGBB` (6 hex digits) → [`Color::Hex`]
/// - `rgb(r, g, b)` with 0-255 each → [`Color::Rgb`]
///
/// Anything else is a parse error (caught at deserialize time).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Color {
    /// A named color (`red`, `blue`, etc.). See [`NamedColor`].
    Named(NamedColor),
    /// Hex color (`#RRGGBB` or `0xRRGGBB`, 6 hex digits).
    Hex(String),
    /// RGB color in `rgb(r, g, b)` syntax.
    Rgb(String),
    /// Default theme-dependent color.
    Default,
}

impl Color {
    /// Serialize back to the original string form.
    #[must_use]
    pub fn to_source_string(&self) -> String {
        match self {
            Self::Default => "default".to_string(),
            Self::Named(n) => n.as_str().to_string(),
            Self::Hex(s) | Self::Rgb(s) => s.clone(),
        }
    }

    /// Try to parse a color from a source string.
    ///
    /// # Errors
    ///
    /// Returns a string description if the input is not a recognized color.
    pub fn parse(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        if trimmed == "default" {
            return Ok(Self::Default);
        }
        if let Some(named) = NamedColor::from_str_lossy(trimmed) {
            return Ok(Self::Named(named));
        }
        // Hex: #RRGGBB or 0xRRGGBB
        let hex_inner = trimmed
            .strip_prefix('#')
            .or_else(|| trimmed.strip_prefix("0x"));
        if let Some(hex) = hex_inner {
            if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Ok(Self::Hex(trimmed.to_string()));
            }
            return Err(format!(
                "invalid hex color `{s}` (expected #RRGGBB or 0xRRGGBB)"
            ));
        }
        // RGB: rgb(r, g, b)
        if let Some(inner) = trimmed
            .strip_prefix("rgb(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
            if parts.len() == 3
                && parts
                    .iter()
                    .all(|p| p.parse::<u32>().map(|n| n <= 255).unwrap_or(false))
            {
                return Ok(Self::Rgb(trimmed.to_string()));
            }
            return Err(format!(
                "invalid rgb color `{s}` (expected rgb(r, g, b) with 0-255 each)"
            ));
        }
        Err(format!(
            "unknown color `{s}` (expected named color, #RRGGBB, 0xRRGGBB, rgb(r,g,b), or default)"
        ))
    }
}

impl TryFrom<String> for Color {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<Color> for String {
    fn from(c: Color) -> String {
        c.to_source_string()
    }
}

/// Standard Discord named colors. See
/// [`docs/SCHEMA.md` §11](../../docs/SCHEMA.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamedColor {
    /// Default
    Default,
    /// White
    White,
    /// Black
    Black,
    /// Dark gray
    DarkGray,
    /// Lighter gray
    LighterGray,
    /// Darker gray
    DarkerGray,
    /// Light gray
    LightGray,
    /// Very dark gray
    VeryDarkGray,
    /// Red
    Red,
    /// Dark red
    DarkRed,
    /// Orange
    Orange,
    /// Dark orange
    DarkOrange,
    /// Gold
    Gold,
    /// Dark gold
    DarkGold,
    /// Yellow
    Yellow,
    /// Dark yellow
    DarkYellow,
    /// Green
    Green,
    /// Dark green
    DarkGreen,
    /// Teal
    Teal,
    /// Dark teal
    DarkTeal,
    /// Blue
    Blue,
    /// Dark blue
    DarkBlue,
    /// Purple
    Purple,
    /// Dark purple
    DarkPurple,
    /// Magenta
    Magenta,
    /// Dark magenta
    DarkMagenta,
    /// Light pink
    LightPink,
    /// Dark pink
    DarkPink,
}

impl NamedColor {
    /// All variants.
    #[must_use]
    pub const fn all() -> &'static [NamedColor] {
        &[
            Self::Default,
            Self::White,
            Self::Black,
            Self::DarkGray,
            Self::LighterGray,
            Self::DarkerGray,
            Self::LightGray,
            Self::VeryDarkGray,
            Self::Red,
            Self::DarkRed,
            Self::Orange,
            Self::DarkOrange,
            Self::Gold,
            Self::DarkGold,
            Self::Yellow,
            Self::DarkYellow,
            Self::Green,
            Self::DarkGreen,
            Self::Teal,
            Self::DarkTeal,
            Self::Blue,
            Self::DarkBlue,
            Self::Purple,
            Self::DarkPurple,
            Self::Magenta,
            Self::DarkMagenta,
            Self::LightPink,
            Self::DarkPink,
        ]
    }

    /// Returns the canonical `snake_case` name as a string slice.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::White => "white",
            Self::Black => "black",
            Self::DarkGray => "dark_gray",
            Self::LighterGray => "lighter_gray",
            Self::DarkerGray => "darker_gray",
            Self::LightGray => "light_gray",
            Self::VeryDarkGray => "very_dark_gray",
            Self::Red => "red",
            Self::DarkRed => "dark_red",
            Self::Orange => "orange",
            Self::DarkOrange => "dark_orange",
            Self::Gold => "gold",
            Self::DarkGold => "dark_gold",
            Self::Yellow => "yellow",
            Self::DarkYellow => "dark_yellow",
            Self::Green => "green",
            Self::DarkGreen => "dark_green",
            Self::Teal => "teal",
            Self::DarkTeal => "dark_teal",
            Self::Blue => "blue",
            Self::DarkBlue => "dark_blue",
            Self::Purple => "purple",
            Self::DarkPurple => "dark_purple",
            Self::Magenta => "magenta",
            Self::DarkMagenta => "dark_magenta",
            Self::LightPink => "light_pink",
            Self::DarkPink => "dark_pink",
        }
    }

    /// Parse a color name from a string.
    ///
    /// # Errors
    ///
    /// Returns `None` if the input is not a recognized color name.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_role_parses() {
        let yaml = "name: Admin\n";
        let r: Role = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(r.name, "Admin");
        assert!(r.color.is_none());
    }

    #[test]
    fn full_role_parses() {
        let yaml = "\
name: Admin
color: red
hoist: true
mentionable: true
permissions: [administrator]
position: 10
unicode_emoji: \"X\"
";
        let r: Role = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(r.color, Some(Color::Named(NamedColor::Red)));
        assert_eq!(r.permissions, vec!["administrator".to_string()]);
        assert_eq!(r.position, Some(10));
    }

    #[test]
    fn unknown_field_rejected() {
        let yaml = "name: Admin\nbogus: true\n";
        let r: Result<Role, _> = serde_yaml::from_str(yaml);
        assert!(r.is_err());
    }

    #[test]
    fn named_color_round_trip() {
        for c in NamedColor::all() {
            let s = c.as_str();
            assert_eq!(NamedColor::from_str_lossy(s), Some(*c));
        }
    }

    #[test]
    fn named_color_rejects_unknown() {
        assert_eq!(NamedColor::from_str_lossy("rainbow"), None);
    }
}
