//! Color theme for the TUI
//!
//! Colors can be configured via a `tui_colors.toml` file in the current working directory.
//! If the file doesn't exist or is invalid, default colors are used.
//!
//! Example `tui_colors.toml`:
//! ```toml
//! light_square = 200, 180, 150  # Medium-light beige
//! dark_square = 120, 90, 60      # Medium-dark brown
//! piece_white = 255, 255, 255    # Very light - visible on dark squares
//! piece_black = 30, 30, 30       # Very dark - visible on light squares
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
        // Try to load from config file, fall back to defaults
        Self::from_config_file().unwrap_or(Self {
            light_square: Color::Rgb(200, 180, 150), // Medium-light beige - provides contrast for both piece colors
            dark_square: Color::Rgb(120, 90, 60), // Medium-dark brown - provides contrast for both piece colors
            piece_white: Color::Rgb(255, 255, 255), // Very light - clearly visible on dark squares
            piece_black: Color::Rgb(30, 30, 30),  // Very dark - clearly visible on light squares
            highlight: Color::Yellow,
            border: Color::Gray,
            text: Color::White,
        })
    }
}

impl Theme {
    /// Load theme from `tui_colors.toml` file in the current working directory.
    /// Returns None if the file doesn't exist or can't be parsed.
    fn from_config_file() -> Option<Self> {
        let config_path = Path::new("tui_colors.toml");
        if !config_path.exists() {
            return None;
        }

        let contents = fs::read_to_string(config_path).ok()?;
        let mut colors = HashMap::new();

        // Parse simple key = value format
        for line in contents.lines() {
            let line = line.trim();
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse "key = r, g, b" format
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                // Parse RGB values: "r, g, b" or "[r, g, b]"
                let rgb_str = value.trim_start_matches('[').trim_end_matches(']');
                let rgb_parts: Vec<&str> = rgb_str.split(',').map(|s| s.trim()).collect();
                if rgb_parts.len() == 3 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        rgb_parts[0].parse::<u8>(),
                        rgb_parts[1].parse::<u8>(),
                        rgb_parts[2].parse::<u8>(),
                    ) {
                        colors.insert(key.to_string(), Color::Rgb(r, g, b));
                    }
                }
            }
        }

        // All four colors must be present
        Some(Self {
            light_square: *colors.get("light_square")?,
            dark_square: *colors.get("dark_square")?,
            piece_white: *colors.get("piece_white")?,
            piece_black: *colors.get("piece_black")?,
            highlight: Color::Yellow, // Not configurable for now
            border: Color::Gray,      // Not configurable for now
            text: Color::White,       // Not configurable for now
        })
    }

    /// Get style for a square with a specific piece color
    pub fn square_style(
        &self,
        is_light_square: bool,
        piece_color: Option<crate::board::color::Color>,
    ) -> Style {
        let square_bg = if is_light_square {
            self.light_square
        } else {
            self.dark_square
        };

        // Use bright foreground colors on square backgrounds for clean, visible pieces
        let style = Style::default().bg(square_bg);

        match piece_color {
            Some(crate::board::color::Color::White) => {
                // White pieces: white foreground, bold for visibility
                style.fg(self.piece_white).add_modifier(Modifier::BOLD)
            }
            Some(crate::board::color::Color::Black) => {
                // Black pieces: dark gray foreground, bold for visibility
                style.fg(self.piece_black).add_modifier(Modifier::BOLD)
            }
            None => {
                // Empty square
                style
            }
        }
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
