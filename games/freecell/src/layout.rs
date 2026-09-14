//! Layout helpers: positions of columns, free cells, foundations.

use glam::Vec2;
use macroquad::prelude::Rect;

use crate::config::GameContext;

/// The four free cells, indexed 0..4, laid out left-to-right.
pub fn free_cell_rect(index: usize, ctx: &GameContext) -> Rect {
    let x = ctx.column_gap + index as f32 * (ctx.card_w + ctx.column_gap);
    let y = ctx.playfield_y() + 10.0;
    Rect::new(x, y, ctx.card_w, ctx.card_h)
}

/// The four foundations, indexed 0..4, laid out right-aligned.
pub fn foundation_rect(index: usize, ctx: &GameContext) -> Rect {
    let total_w = 4.0 * ctx.card_w + 3.0 * ctx.column_gap;
    let x_start = ctx.window_w - ctx.column_gap - total_w;
    let x = x_start + index as f32 * (ctx.card_w + ctx.column_gap);
    let y = ctx.playfield_y() + 10.0;
    Rect::new(x, y, ctx.card_w, ctx.card_h)
}

/// A column slot's top-left corner (the position of the first card).
pub fn column_top(index: usize, ctx: &GameContext) -> Vec2 {
    let x = ctx.column_gap + index as f32 * (ctx.card_w + ctx.column_gap);
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

/// Bounding rect of a column (for hit testing the empty part below the cards).
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
    fn test_free_cells_are_left_aligned() {
        let cx = ctx();
        let first = free_cell_rect(0, &cx);
        assert_eq!(first.x, cx.column_gap);
    }

    #[test]
    fn test_foundations_are_right_aligned() {
        let cx = ctx();
        let last = foundation_rect(3, &cx);
        let right_edge = last.x + last.w;
        let expected = cx.window_w - cx.column_gap;
        assert!((right_edge - expected).abs() < 0.01);
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