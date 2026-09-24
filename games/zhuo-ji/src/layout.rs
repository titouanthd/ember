//! Table layout — all geometry in one place.

use glam::Vec2;

pub const FRAME_SIDE: f32 = 240.0;
pub const FRAME_HALF: f32 = FRAME_SIDE * 0.5;

/// Number of horizontal tile slots a meld occupies (Peng and Gang
/// both use 3 slots; the Gang's 4th tile is stacked above the middle).
pub const MELD_SLOTS: usize = 3;

/// Horizontal gap between two adjacent melds on the same row.
pub const MELD_INTER_GAP: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seat {
    South = 0,
    West = 1,
    North = 2,
    East = 3,
}

impl Seat {
    pub const ALL: [Seat; 4] = [Seat::South, Seat::West, Seat::North, Seat::East];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn from_idx(i: usize) -> Self {
        match i {
            0 => Seat::South,
            1 => Seat::West,
            2 => Seat::North,
            3 => Seat::East,
            _ => Seat::South,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileSize {
    pub w: f32,
    pub h: f32,
}

impl TileSize {
    pub const fn new(w: f32, h: f32) -> Self {
        Self { w, h }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TileSizes {
    pub hand: TileSize,
    pub river: TileSize,
    pub ai_back: TileSize,
    pub ai_meld: TileSize,
}

/// Result of laying out a melds row. `row[i]` holds the 3 horizontal
/// slot positions of meld `i`. `stacks[i]` is `Some(pos)` when meld `i`
/// is a Gang (extra tile position above the middle slot).
#[derive(Debug, Clone)]
pub struct MeldsLayout {
    pub row: Vec<[Vec2; MELD_SLOTS]>,
    pub stacks: Vec<Option<Vec2>>,
}

#[derive(Debug, Clone)]
pub struct TableLayout {
    pub scr_w: f32,
    pub scr_h: f32,

    pub sizes: TileSizes,

    pub hand_gap: f32,
    pub drawn_gap: f32,
    pub meld_gap: f32,

    pub river_cols: usize,
    pub river_rows: usize,
    pub river_gap: f32,

    pub margin: f32,
    pub avatar: f32,

    pub wall: Vec2,
    pub clock_r: f32,

    pub standing_extra: f32,
}

impl TableLayout {
    pub fn new(scr_w: f32, scr_h: f32) -> Self {
        let hand = if scr_w >= 1300.0 {
            TileSize::new(56.0, 84.0)
        } else if scr_w >= 1000.0 {
            TileSize::new(52.0, 80.0)
        } else {
            TileSize::new(48.0, 72.0)
        };

        let river = TileSize::new(30.0, 46.0);
        let ai_back = TileSize::new(30.0, 46.0);
        let ai_meld = TileSize::new(28.0, 42.0);

        Self {
            scr_w,
            scr_h,
            sizes: TileSizes { hand, river, ai_back, ai_meld },
            hand_gap: 4.0,
            drawn_gap: 24.0,
            meld_gap: 24.0,
            river_cols: 6,
            river_rows: 3,
            river_gap: 3.0,
            margin: 20.0,
            avatar: 50.0,
            wall: Vec2::new(170.0, 42.0),
            clock_r: 32.0,
            standing_extra: 12.0,
        }
    }

    pub fn cx(&self) -> f32 {
        self.scr_w * 0.5
    }

    pub fn cy(&self) -> f32 {
        self.scr_h * 0.5
    }

    // ─── Human row ────────────────────────────────────────────────────

    pub fn human_row_y(&self) -> f32 {
        self.scr_h - self.margin - self.sizes.hand.h - self.standing_extra
    }

    pub fn hand_width(&self, tile_count: usize) -> f32 {
        if tile_count == 0 {
            return 0.0;
        }
        tile_count as f32 * self.sizes.hand.w
            + (tile_count - 1) as f32 * self.hand_gap
    }

    pub fn hand_start_x(&self, tile_count: usize) -> f32 {
        self.cx() - self.hand_width(tile_count) * 0.5
    }

    pub fn hand_tile_pos(&self, i: usize, tile_count: usize) -> Vec2 {
        Vec2::new(
            self.hand_start_x(tile_count) + i as f32 * (self.sizes.hand.w + self.hand_gap),
            self.human_row_y(),
        )
    }

    pub fn drawn_pos(&self, tile_count: usize) -> Vec2 {
        Vec2::new(
            self.hand_start_x(tile_count) + self.hand_width(tile_count) + self.drawn_gap,
            self.human_row_y(),
        )
    }

    // ─── Melds row ────────────────────────────────────────────────────

    /// Width of one meld in horizontal slots (3 slots for Peng and Gang).
    pub fn meld_width(&self) -> f32 {
        let t = self.sizes.ai_meld;
        let g = 2.0;
        MELD_SLOTS as f32 * t.w + (MELD_SLOTS as f32 - 1.0) * g
    }

    /// Total width of a melds row with `meld_count` melds.
    pub fn melds_row_width(&self, meld_count: usize) -> f32 {
        if meld_count == 0 {
            return 0.0;
        }
        meld_count as f32 * self.meld_width()
            + (meld_count as f32 - 1.0) * MELD_INTER_GAP
    }

    /// Returns the top-left of the melds row for an AI seat.
    /// East is anchored on the right (melds extend leftward visually
    /// even though the row occupies [origin.x, origin.x + row_w]).
    pub fn ai_melds_origin(
        &self,
        seat: Seat,
        concealed_total: usize,
        meld_count: usize,
    ) -> Vec2 {
        let t = self.sizes.ai_meld;
        let row_w = self.melds_row_width(meld_count);
        match seat {
            Seat::North => {
                let cw = concealed_total as f32 * self.sizes.ai_back.w
                    + (concealed_total.saturating_sub(1)) as f32 * 2.0;
                Vec2::new(
                    self.cx() + cw * 0.5 + 30.0,
                    self.margin + self.avatar + 18.0,
                )
            }
            Seat::West => {
                let ch = concealed_total as f32 * self.sizes.ai_back.h
                    + (concealed_total.saturating_sub(1)) as f32 * 2.0;
                Vec2::new(
                    self.margin + self.avatar + 12.0,
                    self.cy() + ch * 0.5 + 20.0,
                )
            }
            Seat::East => {
                let ch = concealed_total as f32 * self.sizes.ai_back.h
                    + (concealed_total.saturating_sub(1)) as f32 * 2.0;
                let col_top = self.cy() - ch * 0.5;
                let col_right = self.scr_w - self.margin - self.avatar - 12.0;
                Vec2::new(
                    col_right - row_w,
                    col_top - t.h - 20.0,
                )
            }
            Seat::South => Vec2::ZERO,
        }
    }

    /// Returns the top-left of the human melds row (anchored LEFT of
    /// the hand).
    pub fn human_melds_origin(&self, hand_tile_count: usize, meld_count: usize) -> Vec2 {
        let row_w = self.melds_row_width(meld_count);
        let hand_start = self.hand_start_x(hand_tile_count);
        let origin_x = hand_start - self.meld_gap - row_w;
        let y = self.human_row_y()
            + (self.sizes.hand.h - self.sizes.ai_meld.h) * 0.5;
        Vec2::new(origin_x, y)
    }

    /// Lays out a row of melds starting at `origin` (top-left of the
    /// row). The `is_gang` slice flags which melds are Gangs (extra
    /// stacked tile above the middle slot).
    pub fn melds_row_layout(
        &self,
        origin: Vec2,
        is_gang: &[bool],
    ) -> MeldsLayout {
        let t = self.sizes.ai_meld;
        let g = 2.0;
        let mw = self.meld_width();
        let step = mw + MELD_INTER_GAP;

        let mut row = Vec::with_capacity(is_gang.len());
        let mut stacks = Vec::with_capacity(is_gang.len());
        let mut cursor_x = origin.x;

        for &gang in is_gang {
            let mut slots = [Vec2::ZERO; MELD_SLOTS];
            for (slot, s) in slots.iter_mut().enumerate() {
                *s = Vec2::new(
                    cursor_x + slot as f32 * (t.w + g),
                    origin.y,
                );
            }
            let stack = if gang {
                Some(Vec2::new(
                    cursor_x + 1.0 * (t.w + g),
                    origin.y - t.h * 0.55,
                ))
            } else {
                None
            };
            row.push(slots);
            stacks.push(stack);
            cursor_x += step;
        }

        MeldsLayout { row, stacks }
    }

    // ─── Rivers ──────────────────────────────────────────────────────

    pub fn river_row_width(&self) -> f32 {
        self.river_cols as f32 * self.sizes.river.w
            + (self.river_cols - 1) as f32 * self.river_gap
    }

    pub fn river_col_height(&self) -> f32 {
        self.river_cols as f32 * self.sizes.river.h
            + (self.river_cols - 1) as f32 * self.river_gap
    }

    pub fn river_anchor(&self, seat: Seat) -> Vec2 {
        let cx = self.cx();
        let cy = self.cy();
        let rw = self.river_row_width();
        let t = self.sizes.river;

        match seat {
            Seat::South => Vec2::new(cx - rw * 0.5, cy + FRAME_HALF),
            Seat::North => Vec2::new(cx - rw * 0.5, cy - FRAME_HALF - t.h),
            Seat::West => Vec2::new(
                cx - FRAME_HALF - t.w,
                cy - self.river_col_height() * 0.5,
            ),
            Seat::East => Vec2::new(
                cx + FRAME_HALF,
                cy - self.river_col_height() * 0.5,
            ),
        }
    }

    pub fn river_tile_pos(&self, seat: Seat, i: usize) -> Vec2 {
        let anchor = self.river_anchor(seat);
        let row = i / self.river_cols;
        let col = i % self.river_cols;
        let t = self.sizes.river;
        let g = self.river_gap;

        match seat {
            Seat::South => Vec2::new(
                anchor.x + col as f32 * (t.w + g),
                anchor.y + row as f32 * (t.h + g),
            ),
            Seat::North => Vec2::new(
                anchor.x + col as f32 * (t.w + g),
                anchor.y - row as f32 * (t.h + g),
            ),
            Seat::West => Vec2::new(
                anchor.x - row as f32 * (t.w + g),
                anchor.y + col as f32 * (t.h + g),
            ),
            Seat::East => Vec2::new(
                anchor.x + row as f32 * (t.w + g),
                anchor.y + col as f32 * (t.h + g),
            ),
        }
    }

    pub fn river_capacity(&self) -> usize {
        self.river_cols * self.river_rows
    }

    // ─── AI hands ────────────────────────────────────────────────────

    pub fn ai_back_pos(&self, seat: Seat, i: usize, total: usize) -> Vec2 {
        let t = self.sizes.ai_back;
        let g = 2.0;

        match seat {
            Seat::North => {
                let row_w = total as f32 * t.w + (total.saturating_sub(1)) as f32 * g;
                Vec2::new(
                    self.cx() - row_w * 0.5 + i as f32 * (t.w + g),
                    self.margin + self.avatar + 18.0,
                )
            }
            Seat::West | Seat::East => {
                let col_h = total as f32 * t.h + (total.saturating_sub(1)) as f32 * g;
                let base_y = self.cy() - col_h * 0.5;
                let base_x = match seat {
                    Seat::West => self.margin + self.avatar + 12.0,
                    Seat::East => self.scr_w - self.margin - self.avatar - 12.0 - t.w,
                    _ => unreachable!(),
                };
                Vec2::new(base_x, base_y + i as f32 * (t.h + g))
            }
            Seat::South => Vec2::ZERO,
        }
    }

    pub fn ai_drawn_pos(&self, seat: Seat, concealed_total: usize) -> Vec2 {
        let t = self.sizes.ai_back;
        let g = 2.0;
        match seat {
            Seat::North => {
                let row_w = concealed_total as f32 * t.w
                    + (concealed_total.saturating_sub(1)) as f32 * g;
                let base_x = self.cx() - row_w * 0.5;
                Vec2::new(
                    base_x - t.w - 14.0,
                    self.margin + self.avatar + 18.0,
                )
            }
            Seat::West => {
                let col_h = concealed_total as f32 * t.h
                    + (concealed_total.saturating_sub(1)) as f32 * g;
                let base_y = self.cy() - col_h * 0.5;
                Vec2::new(
                    self.margin + self.avatar + 12.0,
                    base_y - t.h - 14.0,
                )
            }
            Seat::East => {
                let col_h = concealed_total as f32 * t.h
                    + (concealed_total.saturating_sub(1)) as f32 * g;
                let base_y = self.cy() - col_h * 0.5;
                Vec2::new(
                    self.scr_w - self.margin - self.avatar - 12.0 - t.w,
                    base_y + col_h + 14.0,
                )
            }
            Seat::South => Vec2::ZERO,
        }
    }

    // ─── Avatars ──────────────────────────────────────────────────────

    pub fn avatar_pos(&self, seat: Seat) -> Vec2 {
        let a = self.avatar;
        match seat {
            Seat::South => Vec2::new(
                self.scr_w - self.margin - a,
                self.human_row_y() + self.sizes.hand.h - a + 4.0,
            ),
            Seat::North => Vec2::new(self.cx() - a * 0.5, self.margin),
            Seat::West => Vec2::new(self.margin, self.cy() - a * 0.5),
            Seat::East => Vec2::new(self.scr_w - self.margin - a, self.cy() - a * 0.5),
        }
    }

    // ─── Wall + clock ─────────────────────────────────────────────────

    pub fn wall_pos(&self) -> Vec2 {
        Vec2::new(
            self.scr_w - self.margin - self.wall.x,
            self.margin + 4.0,
        )
    }

    pub fn clock_pos(&self) -> Vec2 {
        Vec2::new(self.cx(), self.cy())
    }

    // ─── UI ───────────────────────────────────────────────────────────

    pub fn info_pos(&self) -> Vec2 {
        Vec2::new(self.margin, 30.0)
    }

    pub fn buttons_y(&self) -> f32 {
        self.human_row_y() - self.sizes.hand.h - 60.0
    }

    pub fn buttons_right_x(&self) -> f32 {
        self.scr_w - self.margin
    }

    pub fn buttons_center_x(&self) -> f32 {
        self.cx()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l() -> TableLayout {
        TableLayout::new(1440.0, 900.0)
    }

    #[test]
    fn hand_tile_is_larger_than_river_tile() {
        let lay = l();
        assert!(lay.sizes.hand.w > lay.sizes.river.w);
    }

    #[test]
    fn river_matches_ai_back_size() {
        let lay = l();
        assert_eq!(lay.sizes.river, lay.sizes.ai_back);
    }

    #[test]
    fn south_row0_top_at_frame_bottom() {
        let lay = l();
        let p = lay.river_tile_pos(Seat::South, 0);
        assert!((p.y - (lay.cy() + FRAME_HALF)).abs() < 0.5);
    }

    #[test]
    fn north_row0_bottom_at_frame_top() {
        let lay = l();
        let p = lay.river_tile_pos(Seat::North, 0);
        let expected = lay.cy() - FRAME_HALF - lay.sizes.river.h;
        assert!((p.y - expected).abs() < 0.5);
    }

    #[test]
    fn west_row0_right_at_frame_left() {
        let lay = l();
        let p = lay.river_tile_pos(Seat::West, 0);
        let expected = lay.cx() - FRAME_HALF - lay.sizes.river.w;
        assert!((p.x - expected).abs() < 0.5);
    }

    #[test]
    fn east_row0_left_at_frame_right() {
        let lay = l();
        let p = lay.river_tile_pos(Seat::East, 0);
        assert!((p.x - (lay.cx() + FRAME_HALF)).abs() < 0.5);
    }

    #[test]
    fn north_drawn_is_left_of_concealed() {
        let lay = l();
        let concealed_left = lay.ai_back_pos(Seat::North, 0, 13).x;
        let drawn = lay.ai_drawn_pos(Seat::North, 13);
        assert!(drawn.x + lay.sizes.ai_back.w < concealed_left);
    }

    #[test]
    fn west_drawn_is_above_concealed() {
        let lay = l();
        let concealed_top = lay.ai_back_pos(Seat::West, 0, 13).y;
        let drawn = lay.ai_drawn_pos(Seat::West, 13);
        assert!(drawn.y + lay.sizes.ai_back.h < concealed_top);
    }

    #[test]
    fn east_drawn_is_below_concealed() {
        let lay = l();
        let concealed_bottom = lay.ai_back_pos(Seat::East, 12, 13).y + lay.sizes.ai_back.h;
        let drawn = lay.ai_drawn_pos(Seat::East, 13);
        assert!(drawn.y > concealed_bottom);
    }

    // ─── Melds: origin ───────────────────────────────────────────────

    #[test]
    fn north_melds_origin_is_right_of_concealed() {
        let lay = l();
        let concealed_right = lay.ai_back_pos(Seat::North, 12, 13).x + lay.sizes.ai_back.w;
        let origin = lay.ai_melds_origin(Seat::North, 13, 1);
        assert!(origin.x > concealed_right);
    }

    #[test]
    fn west_melds_origin_is_below_concealed() {
        let lay = l();
        let concealed_bottom = lay.ai_back_pos(Seat::West, 12, 13).y + lay.sizes.ai_back.h;
        let origin = lay.ai_melds_origin(Seat::West, 13, 1);
        assert!(origin.y > concealed_bottom);
    }

    #[test]
    fn east_melds_origin_is_above_concealed() {
        let lay = l();
        let concealed_top = lay.ai_back_pos(Seat::East, 0, 13).y;
        let origin = lay.ai_melds_origin(Seat::East, 13, 1);
        assert!(origin.y + lay.sizes.ai_meld.h < concealed_top);
    }

    // ─── Melds: row layout ───────────────────────────────────────────

    #[test]
    fn melds_row_layout_peng_has_no_stack() {
        let lay = l();
        let origin = Vec2::new(0.0, 0.0);
        let layout = lay.melds_row_layout(origin, &[false]);
        assert_eq!(layout.row.len(), 1);
        assert!(layout.stacks[0].is_none());
    }

    #[test]
    fn melds_row_layout_gang_has_stack_above_middle() {
        let lay = l();
        let origin = Vec2::new(0.0, 0.0);
        let layout = lay.melds_row_layout(origin, &[true]);
        let stack = layout.stacks[0].expect("gang should have stack");
        // Stack is above origin.y.
        assert!(stack.y < origin.y);
        // Stack x aligns with slot 1 (middle).
        assert!((stack.x - layout.row[0][1].x).abs() < 0.5);
    }

    #[test]
    fn melds_row_layout_has_gap_between_melds() {
        let lay = l();
        let origin = Vec2::new(0.0, 0.0);
        let layout = lay.melds_row_layout(origin, &[false, false]);
        let m0_right = layout.row[0][2].x + lay.sizes.ai_meld.w;
        let m1_left = layout.row[1][0].x;
        assert!(
            m1_left - m0_right >= MELD_INTER_GAP - 0.1,
            "gap {} should be >= {}",
            m1_left - m0_right,
            MELD_INTER_GAP,
        );
    }

    #[test]
    fn melds_row_layout_slots_are_contiguous_within_meld() {
        let lay = l();
        let origin = Vec2::new(0.0, 0.0);
        let layout = lay.melds_row_layout(origin, &[false]);
        let slots = layout.row[0];
        // slot i+1 is to the right of slot i
        assert!(slots[1].x > slots[0].x);
        assert!(slots[2].x > slots[1].x);
        // Same y
        assert_eq!(slots[0].y, slots[1].y);
        assert_eq!(slots[1].y, slots[2].y);
    }

    #[test]
    fn melds_row_width_matches_layout_extent() {
        let lay = l();
        let origin = Vec2::new(0.0, 0.0);
        let layout = lay.melds_row_layout(origin, &[false, true, false]);
        let last_meld = layout.row.last().unwrap();
        let right_edge = last_meld[2].x + lay.sizes.ai_meld.w;
        let expected = lay.melds_row_width(3);
        assert!((right_edge - expected).abs() < 0.1);
    }

    #[test]
    fn human_melds_origin_is_left_of_hand() {
        let lay = l();
        let hand_start = lay.hand_start_x(13);
        let origin = lay.human_melds_origin(13, 1);
        assert!(origin.x < hand_start);
    }

    // ─── Screen bounds ────────────────────────────────────────────────

    #[test]
    fn all_river_tiles_within_screen() {
        let lay = l();
        for seat in Seat::ALL {
            for i in 0..lay.river_capacity() {
                let p = lay.river_tile_pos(seat, i);
                assert!(p.x >= -5.0 && p.x + lay.sizes.river.w <= lay.scr_w + 5.0);
                assert!(p.y >= -5.0 && p.y + lay.sizes.river.h <= lay.scr_h + 5.0);
            }
        }
    }

    #[test]
    fn avatars_are_inside_screen() {
        let lay = l();
        for seat in Seat::ALL {
            let p = lay.avatar_pos(seat);
            assert!(p.x >= 0.0 && p.x + lay.avatar <= lay.scr_w);
            assert!(p.y >= 0.0 && p.y + lay.avatar <= lay.scr_h);
        }
    }

    #[test]
    fn hand_tile_height_is_84_for_wide_screens() {
        let lay = TableLayout::new(1440.0, 900.0);
        assert_eq!(lay.sizes.hand.h, 84.0);
    }

    #[test]
    fn river_tile_height_is_46() {
        let lay = l();
        assert_eq!(lay.sizes.river.h, 46.0);
    }
}