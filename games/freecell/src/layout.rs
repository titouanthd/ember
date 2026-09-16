//! Layout helpers: positions of columns, free cells, foundations.
//!
//! The whole board (8 columns + gaps) is **centered horizontally** in the
//! window. All zone rects are derived from `board_x_offset`, so changing
//! the window width or the number of columns does not break alignment.

use glam::Vec2;
use macroquad::prelude::Rect;

use crate::config::GameContext;

/// Horizontal offset of the whole board, so it is centered.
///
/// The board width is `8 * card_w + 7 * column_gap`. We assume the
/// window is at least as wide as the board; if it is not, the offset is
/// clamped to 0 and the board starts at the left edge.
fn board_x_offset(ctx: &GameContext) -> f32 {
    let board_w = 8.0 * ctx.card_w + 7.0 * ctx.column_gap;
    ((ctx.window_w - board_w) * 0.5).max(0.0)
}

/// Total board width (8 columns + 7 gaps).
fn board_width(ctx: &GameContext) -> f32 {
    8.0 * ctx.card_w + 7.0 * ctx.column_gap
}

/// The four free cells, indexed 0..4, laid out left-to-right.
pub fn free_cell_rect(index: usize, ctx: &GameContext) -> Rect {
    let x = board_x_offset(ctx) + index as f32 * (ctx.card_w + ctx.column_gap);
    let y = ctx.playfield_y() + 10.0;
    Rect::new(x, y, ctx.card_w, ctx.card_h)
}

/// The four foundations, indexed 0..4, laid out right-aligned inside the
/// board's horizontal span.
pub fn foundation_rect(index: usize, ctx: &GameContext) -> Rect {
    let x_start =
        board_x_offset(ctx) + board_width(ctx) - 4.0 * ctx.card_w - 3.0 * ctx.column_gap;
    let x = x_start + index as f32 * (ctx.card_w + ctx.column_gap);
    let y = ctx.playfield_y() + 10.0;
    Rect::new(x, y, ctx.card_w, ctx.card_h)
}

/// A column slot's top-left corner (the position of the first card).
pub fn column_top(index: usize, ctx: &GameContext) -> Vec2 {
    let x = board_x_offset(ctx) + index as f32 * (ctx.card_w + ctx.column_gap);
    let y = ctx.playfield_y() + ctx.table_offset_y;
    Vec2::new(x, y)
}

/// The rect for a card at position `card_index` in column `column_index`.
pub fn card_rect_in_column(
    column_index: usize,
    card_index: usize,
    ctx: &GameContext,
) -> Rect {
    let top = column_top(column_index, ctx);
    let y = top.y + card_index as f32 * ctx.card_stack_offset;
    Rect::new(top.x, y, ctx.card_w, ctx.card_h)
}

/// Bounding rect of a column (for hit testing the empty part below the
/// cards, and for highlighting the drop target).
pub fn column_bounds(column_index: usize, ctx: &GameContext) -> Rect {
    let top = column_top(column_index, ctx);
    let h = ctx.playfield_h() - ctx.table_offset_y - 10.0;
    Rect::new(top.x, top.y, ctx.card_w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> GameContext {
        crate::config::load_config()
    }

    #[test]
    fn test_free_cells_are_left_of_foundations() {
        let cx = ctx();
        let first_free = free_cell_rect(0, &cx);
        let first_foundation = foundation_rect(0, &cx);
        assert!(
            first_free.x < first_foundation.x,
            "free cells should be left of foundations"
        );
    }

    #[test]
    fn test_free_cells_are_aligned_on_a_row() {
        let cx = ctx();
        let mut prev_y: Option<f32> = None;
        for i in 0..4 {
            let r = free_cell_rect(i, &cx);
            if let Some(y) = prev_y {
                assert!((r.y - y).abs() < 0.01, "free cells should share the same y");
            }
            prev_y = Some(r.y);
        }
    }

    #[test]
    fn test_foundations_are_right_aligned_inside_board() {
        let cx = ctx();
        let board_right = board_x_offset(&cx) + board_width(&cx);
        let last = foundation_rect(3, &cx);
        let right_edge = last.x + last.w;
        assert!(
            (right_edge - board_right).abs() < 0.01,
            "last foundation should end at the right edge of the board"
        );
    }

    #[test]
    fn test_board_is_centered() {
        let cx = ctx();
        let first_col = column_top(0, &cx);
        let last_col = column_top(7, &cx);
        let board_left = first_col.x;
        let board_right = last_col.x + cx.card_w;
        let left_margin = board_left;
        let right_margin = cx.window_w - board_right;
        assert!(
            (left_margin - right_margin).abs() < 1.0,
            "board should be centered: left {} right {}",
            left_margin,
            right_margin
        );
    }

    #[test]
    fn test_columns_are_left_to_right() {
        let cx = ctx();
        let c0 = column_top(0, &cx);
        let c1 = column_top(1, &cx);
        assert!(c1.x > c0.x);
    }

    #[test]
    fn test_card_stack_offset() {
        let cx = ctx();
        let r0 = card_rect_in_column(0, 0, &cx);
        let r1 = card_rect_in_column(0, 1, &cx);
        assert!((r1.y - r0.y - cx.card_stack_offset).abs() < 0.01);
    }
}