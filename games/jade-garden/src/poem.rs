//! The poem《春晓》by Meng Haoran (Tang dynasty, 689–740).
//!
//! Each character has a global index 0-19 (5 per line × 4 lines).
//! The player reveals characters in order, one per level.

pub const POEM_LINES: [&str; 4] = [
    "春眠不觉晓",
    "处处闻啼鸟",
    "夜来风雨声",
    "花落知多少",
];

/// Pinyin (with tones) for each character, in the same order.
pub const POEM_PINYIN: [[&str; 5]; 4] = [
    ["chūn", "mián", "bù", "jué", "xiǎo"],
    ["chù", "chù", "wén", "tí", "niǎo"],
    ["yè", "lái", "fēng", "yǔ", "shēng"],
    ["huā", "luò", "zhī", "duō", "shǎo"],
];

pub const POEM_TITLE: &str = "《春晓》 — Spring Dawn";
pub const POEM_AUTHOR: &str = "Meng Haoran (Tang, 689–740)";

/// Total number of characters in the poem.
pub const TOTAL_CHARS: u8 = 20;

/// Converts a global index (0-19) into (line, column).
pub fn position(index: u8) -> Option<(usize, usize)> {
    if index >= TOTAL_CHARS {
        return None;
    }
    Some(((index as usize) / 5, (index as usize) % 5))
}

/// Character at a global index (0-19), if valid.
pub fn char_at(index: u8) -> Option<char> {
    let (line, col) = position(index)?;
    POEM_LINES[line].chars().nth(col)
}

/// Pinyin at a global index (0-19), if valid.
pub fn pinyin_at(index: u8) -> Option<&'static str> {
    let (line, col) = position(index)?;
    Some(POEM_PINYIN[line][col])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poem_has_exactly_twenty_chars() {
        let total: usize = POEM_LINES.iter().map(|l| l.chars().count()).sum();
        assert_eq!(total, TOTAL_CHARS as usize);
    }

    #[test]
    fn every_line_has_five_chars() {
        for line in POEM_LINES {
            assert_eq!(line.chars().count(), 5);
        }
    }

    #[test]
    fn pinyin_table_matches_poem_shape() {
        for (line, pinyins) in POEM_PINYIN.iter().enumerate() {
            assert_eq!(pinyins.len(), POEM_LINES[line].chars().count());
        }
    }

    #[test]
    fn position_roundtrip() {
        for i in 0..TOTAL_CHARS {
            let (line, col) = position(i).unwrap();
            assert_eq!(line * 5 + col, i as usize);
        }
    }

    #[test]
    fn position_out_of_bounds() {
        assert!(position(20).is_none());
        assert!(position(255).is_none());
    }

    #[test]
    fn char_at_every_index() {
        for i in 0..TOTAL_CHARS {
            assert!(char_at(i).is_some());
        }
    }

    #[test]
    fn first_char_is_chun() {
        assert_eq!(char_at(0), Some('春'));
        assert_eq!(pinyin_at(0), Some("chūn"));
    }

    #[test]
    fn last_char_is_shao() {
        assert_eq!(char_at(19), Some('少'));
        assert_eq!(pinyin_at(19), Some("shǎo"));
    }

    #[test]
    fn poem_starts_with_spring() {
        assert_eq!(POEM_LINES[0].chars().next(), Some('春'));
    }
}