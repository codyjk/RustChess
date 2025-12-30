//! Chess board widget for TUI rendering

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Widget},
};

use common::bitboard::Square;

use crate::board::{color::Color as PieceColor, piece::Piece, Board};
use crate::tui::Theme;

/// Widget that renders a chess board
pub struct BoardWidget<'a> {
    board: &'a Board,
    theme: &'a Theme,
}

impl<'a> BoardWidget<'a> {
    pub fn new(board: &'a Board, theme: &'a Theme) -> Self {
        Self { board, theme }
    }

    fn get_piece_char(piece: Piece, color: PieceColor) -> char {
        piece.to_unicode_piece_char(color)
    }
}

impl Widget for BoardWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create a bordered block for the board
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Chess Board")
            .border_style(self.theme.border_style());

        let inner = block.inner(area);
        block.render(area, buf);

        // Calculate square dimensions based on available space
        // We need: 1 char for rank labels + 8 squares + file labels (top/bottom)
        // Minimum 2 chars per square for piece visibility
        let available_width = inner.width.saturating_sub(1); // Reserve 1 for rank labels
        let available_height = inner.height.saturating_sub(2); // Reserve 2 for file labels (top/bottom)

        // Calculate square size (min 2, max 6 for readability)
        let square_width = (available_width / 8).clamp(2, 6);
        let square_height = (available_height / 8).clamp(1, 3);

        // Check if we have enough space
        if square_width < 2 || square_height < 1 {
            return; // Not enough space to render
        }

        // Render file labels (a-h) at top, center-aligned
        for file in 0u8..8 {
            let x = inner.x + 1 + (u16::from(file) * square_width) + square_width / 2;
            let y = inner.y;
            if x < inner.x + inner.width && y < inner.y + inner.height {
                buf.cell_mut((x, y))
                    .unwrap()
                    .set_char((b'a' + file) as char)
                    .set_style(self.theme.text_style());
            }
        }

        // Render board squares (from rank 7 down to 0)
        for rank in 0u8..8 {
            let display_rank = 7 - rank; // Display from top (rank 8) to bottom (rank 1)
            let y = inner.y + 1 + (u16::from(rank) * square_height);

            // Render rank label (8 down to 1), center-aligned
            let label_y = y + square_height / 2;
            if label_y < inner.y + inner.height {
                buf.cell_mut((inner.x, label_y))
                    .unwrap()
                    .set_char((b'8' - rank) as char)
                    .set_style(self.theme.text_style());
            }

            // Render squares for this rank
            for file in 0u8..8 {
                let square = Square::from_rank_file(display_rank, file);
                let x = inner.x + 1 + (u16::from(file) * square_width);

                if x + square_width <= inner.x + inner.width && y < inner.y + inner.height {
                    // Determine square color (alternating pattern)
                    let is_light = (display_rank + file) % 2 == 0;

                    // Get piece on this square
                    let (piece_char, piece_color) = match self.board.get(square) {
                        Some((piece, color)) => (Self::get_piece_char(piece, color), Some(color)),
                        None => (' ', None),
                    };

                    let square_style = self.theme.square_style(is_light, piece_color);

                    // Render the square with dynamic width and height
                    for dy in 0..square_height {
                        for dx in 0..square_width {
                            let cell_x = x + dx;
                            let cell_y = y + dy;
                            if cell_x < inner.x + inner.width && cell_y < inner.y + inner.height {
                                // Place piece character in center of square
                                let is_center = dx == square_width / 2 && dy == square_height / 2;
                                let ch = if is_center { piece_char } else { ' ' };

                                buf.cell_mut((cell_x, cell_y))
                                    .unwrap()
                                    .set_char(ch)
                                    .set_style(square_style);
                            }
                        }
                    }
                }
            }
        }

        // Render file labels (a-h) at bottom, center-aligned
        for file in 0u8..8 {
            let x = inner.x + 1 + (u16::from(file) * square_width) + square_width / 2;
            let y = inner.y + 1 + (8 * square_height);
            if x < inner.x + inner.width && y < inner.y + inner.height {
                buf.cell_mut((x, y))
                    .unwrap()
                    .set_char((b'a' + file) as char)
                    .set_style(self.theme.text_style());
            }
        }
    }
}
