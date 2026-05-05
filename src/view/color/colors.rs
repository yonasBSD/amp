use crate::view::color::RGBColor;

/// A convenience type used to represent a foreground/background
/// color combination. Provides generic/convenience variants to
/// discourage color selection outside of the theme, whenever possible.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Colors {
    #[default]
    Default, // default foreground on the default background
    Focused,     // default foreground on the focused background
    Inverted,    // default, just inverted
    Insert,      // white/green
    Warning,     // white/yellow
    PinnedQuery, // white/blue
    PasteMode,   // white/purple
    PathMode,    // white/pink
    SearchMode,  // white/purple
    SelectMode,  // white/blue
    // Custom foreground on the default background
    CustomForeground(RGBColor),
    // Custom foreground on the focused background
    CustomFocusedForeground(RGBColor),
    // Fully resolved colors; used when a specific background must be preserved
    Custom(RGBColor, RGBColor),
}
