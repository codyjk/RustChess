//! Color theme for the TUI

use ratatui::style::{Color, Modifier, Style};

/// Color theme for the chess TUI
pub struct Theme {
    pub light_square: Color,
    pub dark_square: Color,
    pub piece_white: Color,
    pub piece_black: Color,
    pub highlight: Color,
    pub border: Color,
    pub text: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            light_square: Color::Rgb(240, 217, 181), // Wheat
            dark_square: Color::Rgb(181, 136, 99),   // Sienna
            piece_white: Color::White,
            piece_black: Color::Black,
            highlight: Color::Yellow,
            border: Color::Gray,
            text: Color::White,
        }
    }
}

impl Theme {
    /// Get style for a square with a specific piece color
    pub fn square_style(
        &self,
        is_light_square: bool,
        piece_color: Option<crate::board::color::Color>,
    ) -> Style {
        let bg = if is_light_square {
            self.light_square
        } else {
            self.dark_square
        };

        let fg = match piece_color {
            Some(crate::board::color::Color::White) => self.piece_white,
            Some(crate::board::color::Color::Black) => self.piece_black,
            None => bg, // Empty square, fg doesn't matter
        };

        // Make pieces bold for better visibility
        Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD)
    }

    /// Get style for text
    pub fn text_style(&self) -> Style {
        Style::default().fg(self.text)
    }

    /// Get style for borders
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }
}
