//! Named colors (the CSS/X11 set), a series palette and color parsing.

use rgb::RGB8;

/// `#800000`
pub const MAROON: RGB8 = RGB8::new(128, 0, 0);
/// `#8B0000`
pub const DARK_RED: RGB8 = RGB8::new(139, 0, 0);
/// `#A52A2A`
pub const BROWN: RGB8 = RGB8::new(165, 42, 42);
/// `#B22222`
pub const FIREBRICK: RGB8 = RGB8::new(178, 34, 34);
/// `#DC143C`
pub const CRIMSON: RGB8 = RGB8::new(220, 20, 60);
/// `#FF0000`
pub const RED: RGB8 = RGB8::new(255, 0, 0);
/// `#FF6347`
pub const TOMATO: RGB8 = RGB8::new(255, 99, 71);
/// `#FF7F50`
pub const CORAL: RGB8 = RGB8::new(255, 127, 80);
/// `#CD5C5C`
pub const INDIAN_RED: RGB8 = RGB8::new(205, 92, 92);
/// `#F08080`
pub const LIGHT_CORAL: RGB8 = RGB8::new(240, 128, 128);
/// `#E9967A`
pub const DARK_SALMON: RGB8 = RGB8::new(233, 150, 122);
/// `#FA8072`
pub const SALMON: RGB8 = RGB8::new(250, 128, 114);
/// `#FFA07A`
pub const LIGHT_SALMON: RGB8 = RGB8::new(255, 160, 122);
/// `#FF4500`
pub const ORANGE_RED: RGB8 = RGB8::new(255, 69, 0);
/// `#FF8C00`
pub const DARK_ORANGE: RGB8 = RGB8::new(255, 140, 0);
/// `#FFA500`
pub const ORANGE: RGB8 = RGB8::new(255, 165, 0);
/// `#FFD700`
pub const GOLD: RGB8 = RGB8::new(255, 215, 0);
/// `#B8860B`
pub const DARK_GOLDEN_ROD: RGB8 = RGB8::new(184, 134, 11);
/// `#DAA520`
pub const GOLDEN_ROD: RGB8 = RGB8::new(218, 165, 32);
/// `#EEE8AA`
pub const PALE_GOLDEN_ROD: RGB8 = RGB8::new(238, 232, 170);
/// `#BDB76B`
pub const DARK_KHAKI: RGB8 = RGB8::new(189, 183, 107);
/// `#F0E68C`
pub const KHAKI: RGB8 = RGB8::new(240, 230, 140);
/// `#808000`
pub const OLIVE: RGB8 = RGB8::new(128, 128, 0);
/// `#FFFF00`
pub const YELLOW: RGB8 = RGB8::new(255, 255, 0);
/// `#9ACD32`
pub const YELLOW_GREEN: RGB8 = RGB8::new(154, 205, 50);
/// `#556B2F`
pub const DARK_OLIVE_GREEN: RGB8 = RGB8::new(85, 107, 47);
/// `#6B8E23`
pub const OLIVE_DRAB: RGB8 = RGB8::new(107, 142, 35);
/// `#7CFC00`
pub const LAWN_GREEN: RGB8 = RGB8::new(124, 252, 0);
/// `#7FFF00`
pub const CHARTREUSE: RGB8 = RGB8::new(127, 255, 0);
/// `#ADFF2F`
pub const GREEN_YELLOW: RGB8 = RGB8::new(173, 255, 47);
/// `#006400`
pub const DARK_GREEN: RGB8 = RGB8::new(0, 100, 0);
/// `#008000`
pub const GREEN: RGB8 = RGB8::new(0, 128, 0);
/// `#228B22`
pub const FOREST_GREEN: RGB8 = RGB8::new(34, 139, 34);
/// `#00FF00`
pub const LIME: RGB8 = RGB8::new(0, 255, 0);
/// `#32CD32`
pub const LIME_GREEN: RGB8 = RGB8::new(50, 205, 50);
/// `#90EE90`
pub const LIGHT_GREEN: RGB8 = RGB8::new(144, 238, 144);
/// `#98FB98`
pub const PALE_GREEN: RGB8 = RGB8::new(152, 251, 152);
/// `#8FBC8F`
pub const DARK_SEA_GREEN: RGB8 = RGB8::new(143, 188, 143);
/// `#00FA9A`
pub const MEDIUM_SPRING_GREEN: RGB8 = RGB8::new(0, 250, 154);
/// `#00FF7F`
pub const SPRING_GREEN: RGB8 = RGB8::new(0, 255, 127);
/// `#2E8B57`
pub const SEA_GREEN: RGB8 = RGB8::new(46, 139, 87);
/// `#66CDAA`
pub const MEDIUM_AQUA_MARINE: RGB8 = RGB8::new(102, 205, 170);
/// `#3CB371`
pub const MEDIUM_SEA_GREEN: RGB8 = RGB8::new(60, 179, 113);
/// `#20B2AA`
pub const LIGHT_SEA_GREEN: RGB8 = RGB8::new(32, 178, 170);
/// `#2F4F4F`
pub const DARK_SLATE_GRAY: RGB8 = RGB8::new(47, 79, 79);
/// `#008080`
pub const TEAL: RGB8 = RGB8::new(0, 128, 128);
/// `#008B8B`
pub const DARK_CYAN: RGB8 = RGB8::new(0, 139, 139);
/// `#00FFFF`
pub const AQUA: RGB8 = RGB8::new(0, 255, 255);
/// `#00FFFF`
pub const CYAN: RGB8 = RGB8::new(0, 255, 255);
/// `#E0FFFF`
pub const LIGHT_CYAN: RGB8 = RGB8::new(224, 255, 255);
/// `#00CED1`
pub const DARK_TURQUOISE: RGB8 = RGB8::new(0, 206, 209);
/// `#40E0D0`
pub const TURQUOISE: RGB8 = RGB8::new(64, 224, 208);
/// `#48D1CC`
pub const MEDIUM_TURQUOISE: RGB8 = RGB8::new(72, 209, 204);
/// `#AFEEEE`
pub const PALE_TURQUOISE: RGB8 = RGB8::new(175, 238, 238);
/// `#7FFFD4`
pub const AQUA_MARINE: RGB8 = RGB8::new(127, 255, 212);
/// `#B0E0E6`
pub const POWDER_BLUE: RGB8 = RGB8::new(176, 224, 230);
/// `#5F9EA0`
pub const CADET_BLUE: RGB8 = RGB8::new(95, 158, 160);
/// `#4682B4`
pub const STEEL_BLUE: RGB8 = RGB8::new(70, 130, 180);
/// `#6495ED`
pub const CORN_FLOWER_BLUE: RGB8 = RGB8::new(100, 149, 237);
/// `#00BFFF`
pub const DEEP_SKY_BLUE: RGB8 = RGB8::new(0, 191, 255);
/// `#1E90FF`
pub const DODGER_BLUE: RGB8 = RGB8::new(30, 144, 255);
/// `#ADD8E6`
pub const LIGHT_BLUE: RGB8 = RGB8::new(173, 216, 230);
/// `#87CEEB`
pub const SKY_BLUE: RGB8 = RGB8::new(135, 206, 235);
/// `#87CEFA`
pub const LIGHT_SKY_BLUE: RGB8 = RGB8::new(135, 206, 250);
/// `#191970`
pub const MIDNIGHT_BLUE: RGB8 = RGB8::new(25, 25, 112);
/// `#000080`
pub const NAVY: RGB8 = RGB8::new(0, 0, 128);
/// `#00008B`
pub const DARK_BLUE: RGB8 = RGB8::new(0, 0, 139);
/// `#0000CD`
pub const MEDIUM_BLUE: RGB8 = RGB8::new(0, 0, 205);
/// `#0000FF`
pub const BLUE: RGB8 = RGB8::new(0, 0, 255);
/// `#4169E1`
pub const ROYAL_BLUE: RGB8 = RGB8::new(65, 105, 225);
/// `#8A2BE2`
pub const BLUE_VIOLET: RGB8 = RGB8::new(138, 43, 226);
/// `#4B0082`
pub const INDIGO: RGB8 = RGB8::new(75, 0, 130);
/// `#483D8B`
pub const DARK_SLATE_BLUE: RGB8 = RGB8::new(72, 61, 139);
/// `#6A5ACD`
pub const SLATE_BLUE: RGB8 = RGB8::new(106, 90, 205);
/// `#7B68EE`
pub const MEDIUM_SLATE_BLUE: RGB8 = RGB8::new(123, 104, 238);
/// `#9370DB`
pub const MEDIUM_PURPLE: RGB8 = RGB8::new(147, 112, 219);
/// `#8B008B`
pub const DARK_MAGENTA: RGB8 = RGB8::new(139, 0, 139);
/// `#9400D3`
pub const DARK_VIOLET: RGB8 = RGB8::new(148, 0, 211);
/// `#9932CC`
pub const DARK_ORCHID: RGB8 = RGB8::new(153, 50, 204);
/// `#BA55D3`
pub const MEDIUM_ORCHID: RGB8 = RGB8::new(186, 85, 211);
/// `#800080`
pub const PURPLE: RGB8 = RGB8::new(128, 0, 128);
/// `#D8BFD8`
pub const THISTLE: RGB8 = RGB8::new(216, 191, 216);
/// `#DDA0DD`
pub const PLUM: RGB8 = RGB8::new(221, 160, 221);
/// `#EE82EE`
pub const VIOLET: RGB8 = RGB8::new(238, 130, 238);
/// `#FF00FF`
pub const MAGENTA: RGB8 = RGB8::new(255, 0, 255);
/// `#DA70D6`
pub const ORCHID: RGB8 = RGB8::new(218, 112, 214);
/// `#C71585`
pub const MEDIUM_VIOLET_RED: RGB8 = RGB8::new(199, 21, 133);
/// `#DB7093`
pub const PALE_VIOLET_RED: RGB8 = RGB8::new(219, 112, 147);
/// `#FF1493`
pub const DEEP_PINK: RGB8 = RGB8::new(255, 20, 147);
/// `#FF69B4`
pub const HOT_PINK: RGB8 = RGB8::new(255, 105, 180);
/// `#FFB6C1`
pub const LIGHT_PINK: RGB8 = RGB8::new(255, 182, 193);
/// `#FFC0CB`
pub const PINK: RGB8 = RGB8::new(255, 192, 203);
/// `#FAEBD7`
pub const ANTIQUE_WHITE: RGB8 = RGB8::new(250, 235, 215);
/// `#F5F5DC`
pub const BEIGE: RGB8 = RGB8::new(245, 245, 220);
/// `#FFE4C4`
pub const BISQUE: RGB8 = RGB8::new(255, 228, 196);
/// `#FFEBCD`
pub const BLANCHED_ALMOND: RGB8 = RGB8::new(255, 235, 205);
/// `#F5DEB3`
pub const WHEAT: RGB8 = RGB8::new(245, 222, 179);
/// `#FFF8DC`
pub const CORN_SILK: RGB8 = RGB8::new(255, 248, 220);
/// `#FFFACD`
pub const LEMON_CHIFFON: RGB8 = RGB8::new(255, 250, 205);
/// `#FAFAD2`
pub const LIGHT_GOLDEN_ROD_YELLOW: RGB8 = RGB8::new(250, 250, 210);
/// `#FFFFE0`
pub const LIGHT_YELLOW: RGB8 = RGB8::new(255, 255, 224);
/// `#8B4513`
pub const SADDLE_BROWN: RGB8 = RGB8::new(139, 69, 19);
/// `#A0522D`
pub const SIENNA: RGB8 = RGB8::new(160, 82, 45);
/// `#D2691E`
pub const CHOCOLATE: RGB8 = RGB8::new(210, 105, 30);
/// `#CD853F`
pub const PERU: RGB8 = RGB8::new(205, 133, 63);
/// `#F4A460`
pub const SANDY_BROWN: RGB8 = RGB8::new(244, 164, 96);
/// `#DEB887`
pub const BURLY_WOOD: RGB8 = RGB8::new(222, 184, 135);
/// `#D2B48C`
pub const TAN: RGB8 = RGB8::new(210, 180, 140);
/// `#BC8F8F`
pub const ROSY_BROWN: RGB8 = RGB8::new(188, 143, 143);
/// `#FFE4B5`
pub const MOCCASIN: RGB8 = RGB8::new(255, 228, 181);
/// `#FFDEAD`
pub const NAVAJO_WHITE: RGB8 = RGB8::new(255, 222, 173);
/// `#FFDAB9`
pub const PEACH_PUFF: RGB8 = RGB8::new(255, 218, 185);
/// `#FFE4E1`
pub const MISTY_ROSE: RGB8 = RGB8::new(255, 228, 225);
/// `#FFF0F5`
pub const LAVENDER_BLUSH: RGB8 = RGB8::new(255, 240, 245);
/// `#FAF0E6`
pub const LINEN: RGB8 = RGB8::new(250, 240, 230);
/// `#FDF5E6`
pub const OLD_LACE: RGB8 = RGB8::new(253, 245, 230);
/// `#FFEFD5`
pub const PAPAYA_WHIP: RGB8 = RGB8::new(255, 239, 213);
/// `#FFF5EE`
pub const SEA_SHELL: RGB8 = RGB8::new(255, 245, 238);
/// `#F5FFFA`
pub const MINT_CREAM: RGB8 = RGB8::new(245, 255, 250);
/// `#708090`
pub const SLATE_GRAY: RGB8 = RGB8::new(112, 128, 144);
/// `#778899`
pub const LIGHT_SLATE_GRAY: RGB8 = RGB8::new(119, 136, 153);
/// `#B0C4DE`
pub const LIGHT_STEEL_BLUE: RGB8 = RGB8::new(176, 196, 222);
/// `#E6E6FA`
pub const LAVENDER: RGB8 = RGB8::new(230, 230, 250);
/// `#FFFAF0`
pub const FLORAL_WHITE: RGB8 = RGB8::new(255, 250, 240);
/// `#F0F8FF`
pub const ALICE_BLUE: RGB8 = RGB8::new(240, 248, 255);
/// `#F8F8FF`
pub const GHOST_WHITE: RGB8 = RGB8::new(248, 248, 255);
/// `#F0FFF0`
pub const HONEYDEW: RGB8 = RGB8::new(240, 255, 240);
/// `#FFFFF0`
pub const IVORY: RGB8 = RGB8::new(255, 255, 240);
/// `#F0FFFF`
pub const AZURE: RGB8 = RGB8::new(240, 255, 255);
/// `#FFFAFA`
pub const SNOW: RGB8 = RGB8::new(255, 250, 250);
/// `#000000`
pub const BLACK: RGB8 = RGB8::new(0, 0, 0);
/// `#696969`
pub const DIM_GRAY: RGB8 = RGB8::new(105, 105, 105);
/// `#808080`
pub const GRAY: RGB8 = RGB8::new(128, 128, 128);
/// `#A9A9A9`
pub const DARK_GRAY: RGB8 = RGB8::new(169, 169, 169);
/// `#C0C0C0`
pub const SILVER: RGB8 = RGB8::new(192, 192, 192);
/// `#D3D3D3`
pub const LIGHT_GRAY: RGB8 = RGB8::new(211, 211, 211);
/// `#DCDCDC`
pub const GAINSBORO: RGB8 = RGB8::new(220, 220, 220);
/// `#F5F5F5`
pub const WHITE_SMOKE: RGB8 = RGB8::new(245, 245, 245);
/// `#FFFFFF`
pub const WHITE: RGB8 = RGB8::new(255, 255, 255);

const COLOR_TABLE: &[(&str, RGB8)] = &[
    ("MAROON", MAROON),
    ("DARK_RED", DARK_RED),
    ("BROWN", BROWN),
    ("FIREBRICK", FIREBRICK),
    ("CRIMSON", CRIMSON),
    ("RED", RED),
    ("TOMATO", TOMATO),
    ("CORAL", CORAL),
    ("INDIAN_RED", INDIAN_RED),
    ("LIGHT_CORAL", LIGHT_CORAL),
    ("DARK_SALMON", DARK_SALMON),
    ("SALMON", SALMON),
    ("LIGHT_SALMON", LIGHT_SALMON),
    ("ORANGE_RED", ORANGE_RED),
    ("DARK_ORANGE", DARK_ORANGE),
    ("ORANGE", ORANGE),
    ("GOLD", GOLD),
    ("DARK_GOLDEN_ROD", DARK_GOLDEN_ROD),
    ("GOLDEN_ROD", GOLDEN_ROD),
    ("PALE_GOLDEN_ROD", PALE_GOLDEN_ROD),
    ("DARK_KHAKI", DARK_KHAKI),
    ("KHAKI", KHAKI),
    ("OLIVE", OLIVE),
    ("YELLOW", YELLOW),
    ("YELLOW_GREEN", YELLOW_GREEN),
    ("DARK_OLIVE_GREEN", DARK_OLIVE_GREEN),
    ("OLIVE_DRAB", OLIVE_DRAB),
    ("LAWN_GREEN", LAWN_GREEN),
    ("CHARTREUSE", CHARTREUSE),
    ("GREEN_YELLOW", GREEN_YELLOW),
    ("DARK_GREEN", DARK_GREEN),
    ("GREEN", GREEN),
    ("FOREST_GREEN", FOREST_GREEN),
    ("LIME", LIME),
    ("LIME_GREEN", LIME_GREEN),
    ("LIGHT_GREEN", LIGHT_GREEN),
    ("PALE_GREEN", PALE_GREEN),
    ("DARK_SEA_GREEN", DARK_SEA_GREEN),
    ("MEDIUM_SPRING_GREEN", MEDIUM_SPRING_GREEN),
    ("SPRING_GREEN", SPRING_GREEN),
    ("SEA_GREEN", SEA_GREEN),
    ("MEDIUM_AQUA_MARINE", MEDIUM_AQUA_MARINE),
    ("MEDIUM_SEA_GREEN", MEDIUM_SEA_GREEN),
    ("LIGHT_SEA_GREEN", LIGHT_SEA_GREEN),
    ("DARK_SLATE_GRAY", DARK_SLATE_GRAY),
    ("TEAL", TEAL),
    ("DARK_CYAN", DARK_CYAN),
    ("AQUA", AQUA),
    ("CYAN", CYAN),
    ("LIGHT_CYAN", LIGHT_CYAN),
    ("DARK_TURQUOISE", DARK_TURQUOISE),
    ("TURQUOISE", TURQUOISE),
    ("MEDIUM_TURQUOISE", MEDIUM_TURQUOISE),
    ("PALE_TURQUOISE", PALE_TURQUOISE),
    ("AQUA_MARINE", AQUA_MARINE),
    ("POWDER_BLUE", POWDER_BLUE),
    ("CADET_BLUE", CADET_BLUE),
    ("STEEL_BLUE", STEEL_BLUE),
    ("CORN_FLOWER_BLUE", CORN_FLOWER_BLUE),
    ("DEEP_SKY_BLUE", DEEP_SKY_BLUE),
    ("DODGER_BLUE", DODGER_BLUE),
    ("LIGHT_BLUE", LIGHT_BLUE),
    ("SKY_BLUE", SKY_BLUE),
    ("LIGHT_SKY_BLUE", LIGHT_SKY_BLUE),
    ("MIDNIGHT_BLUE", MIDNIGHT_BLUE),
    ("NAVY", NAVY),
    ("DARK_BLUE", DARK_BLUE),
    ("MEDIUM_BLUE", MEDIUM_BLUE),
    ("BLUE", BLUE),
    ("ROYAL_BLUE", ROYAL_BLUE),
    ("BLUE_VIOLET", BLUE_VIOLET),
    ("INDIGO", INDIGO),
    ("DARK_SLATE_BLUE", DARK_SLATE_BLUE),
    ("SLATE_BLUE", SLATE_BLUE),
    ("MEDIUM_SLATE_BLUE", MEDIUM_SLATE_BLUE),
    ("MEDIUM_PURPLE", MEDIUM_PURPLE),
    ("DARK_MAGENTA", DARK_MAGENTA),
    ("DARK_VIOLET", DARK_VIOLET),
    ("DARK_ORCHID", DARK_ORCHID),
    ("MEDIUM_ORCHID", MEDIUM_ORCHID),
    ("PURPLE", PURPLE),
    ("THISTLE", THISTLE),
    ("PLUM", PLUM),
    ("VIOLET", VIOLET),
    ("MAGENTA", MAGENTA),
    ("ORCHID", ORCHID),
    ("MEDIUM_VIOLET_RED", MEDIUM_VIOLET_RED),
    ("PALE_VIOLET_RED", PALE_VIOLET_RED),
    ("DEEP_PINK", DEEP_PINK),
    ("HOT_PINK", HOT_PINK),
    ("LIGHT_PINK", LIGHT_PINK),
    ("PINK", PINK),
    ("ANTIQUE_WHITE", ANTIQUE_WHITE),
    ("BEIGE", BEIGE),
    ("BISQUE", BISQUE),
    ("BLANCHED_ALMOND", BLANCHED_ALMOND),
    ("WHEAT", WHEAT),
    ("CORN_SILK", CORN_SILK),
    ("LEMON_CHIFFON", LEMON_CHIFFON),
    ("LIGHT_GOLDEN_ROD_YELLOW", LIGHT_GOLDEN_ROD_YELLOW),
    ("LIGHT_YELLOW", LIGHT_YELLOW),
    ("SADDLE_BROWN", SADDLE_BROWN),
    ("SIENNA", SIENNA),
    ("CHOCOLATE", CHOCOLATE),
    ("PERU", PERU),
    ("SANDY_BROWN", SANDY_BROWN),
    ("BURLY_WOOD", BURLY_WOOD),
    ("TAN", TAN),
    ("ROSY_BROWN", ROSY_BROWN),
    ("MOCCASIN", MOCCASIN),
    ("NAVAJO_WHITE", NAVAJO_WHITE),
    ("PEACH_PUFF", PEACH_PUFF),
    ("MISTY_ROSE", MISTY_ROSE),
    ("LAVENDER_BLUSH", LAVENDER_BLUSH),
    ("LINEN", LINEN),
    ("OLD_LACE", OLD_LACE),
    ("PAPAYA_WHIP", PAPAYA_WHIP),
    ("SEA_SHELL", SEA_SHELL),
    ("MINT_CREAM", MINT_CREAM),
    ("SLATE_GRAY", SLATE_GRAY),
    ("LIGHT_SLATE_GRAY", LIGHT_SLATE_GRAY),
    ("LIGHT_STEEL_BLUE", LIGHT_STEEL_BLUE),
    ("LAVENDER", LAVENDER),
    ("FLORAL_WHITE", FLORAL_WHITE),
    ("ALICE_BLUE", ALICE_BLUE),
    ("GHOST_WHITE", GHOST_WHITE),
    ("HONEYDEW", HONEYDEW),
    ("IVORY", IVORY),
    ("AZURE", AZURE),
    ("SNOW", SNOW),
    ("BLACK", BLACK),
    ("DIM_GRAY", DIM_GRAY),
    ("GRAY", GRAY),
    ("DARK_GRAY", DARK_GRAY),
    ("SILVER", SILVER),
    ("LIGHT_GRAY", LIGHT_GRAY),
    ("GAINSBORO", GAINSBORO),
    ("WHITE_SMOKE", WHITE_SMOKE),
    ("WHITE", WHITE),
];

/// Looks up a named color, ignoring case, underscores, hyphens and spaces, so "DarkRed",
/// "dark-red", "dark red" and "DARK_RED" all match.
pub fn from_name(name: &str) -> Option<RGB8> {
    let wanted = normalize(name);
    COLOR_TABLE
        .iter()
        .find(|(n, _)| normalize(n) == wanted)
        .map(|(_, c)| *c)
}

/// Parses a color name (see [`from_name`]) or a hex color: `#RRGGBB` or `#RGB` (the `#` is
/// optional).
pub fn parse(value: &str) -> Option<RGB8> {
    from_name(value).or_else(|| parse_hex(value.trim()))
}

fn parse_hex(value: &str) -> Option<RGB8> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize, len: usize| u8::from_str_radix(&hex[i..i + len], 16).ok();
    match hex.len() {
        6 => Some(RGB8::new(channel(0, 2)?, channel(2, 2)?, channel(4, 2)?)),
        // #RGB expands each digit, e.g. #f80 -> #ff8800
        3 => Some(RGB8::new(
            channel(0, 1)? * 17,
            channel(1, 1)? * 17,
            channel(2, 1)? * 17,
        )),
        _ => None,
    }
}

fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '_' | '-' | ' '))
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Distinct colors that read well on dark and light backgrounds, used in order for successive
/// series.
pub const PALETTE: [RGB8; 6] = [DODGER_BLUE, RED, LIME, ORANGE, CYAN, MAGENTA];

/// Perceived brightness (Rec. 709 luma) from 0 (black) to 255 (white).
pub fn luminance(color: RGB8) -> f64 {
    0.2126 * color.r as f64 + 0.7152 * color.g as f64 + 0.0722 * color.b as f64
}

/// Returns all available color names (uppercase with underscores).
pub fn all_names() -> &'static [(&'static str, RGB8)] {
    COLOR_TABLE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_ignores_separators() {
        for name in ["DarkRed", "dark-red", "dark red", "DARK_RED", "darkred"] {
            assert_eq!(from_name(name), Some(DARK_RED), "{name}");
        }
    }

    #[test]
    fn normalized_names_are_unique() {
        let mut names: Vec<String> = COLOR_TABLE.iter().map(|(n, _)| normalize(n)).collect();
        let total = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), total);
    }

    #[test]
    fn parse_accepts_names_and_hex() {
        assert_eq!(parse("lime"), Some(LIME));
        assert_eq!(parse("#ff8800"), Some(RGB8::new(255, 136, 0)));
        assert_eq!(parse("FF8800"), Some(RGB8::new(255, 136, 0)));
        assert_eq!(parse("#f80"), Some(RGB8::new(255, 136, 0)));
        assert_eq!(parse("#ff88"), None);
        assert_eq!(parse("#gg0000"), None);
        assert_eq!(parse("nope"), None);
    }

    #[test]
    fn from_name_exact_match() {
        assert_eq!(from_name("BLUE"), Some(BLUE));
        assert_eq!(from_name("RED"), Some(RED));
    }

    #[test]
    fn from_name_case_insensitive() {
        assert_eq!(from_name("blue"), Some(BLUE));
        assert_eq!(from_name("Blue"), Some(BLUE));
        assert_eq!(from_name("dark_red"), Some(DARK_RED));
        assert_eq!(from_name("Dark_Red"), Some(DARK_RED));
    }

    #[test]
    fn from_name_unknown_returns_none() {
        assert_eq!(from_name("Gren"), None);
        assert_eq!(from_name("not_a_color"), None);
        assert_eq!(from_name(""), None);
    }

    #[test]
    fn all_names_is_nonempty() {
        assert!(all_names().len() > 100);
    }
}
