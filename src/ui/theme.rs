//! Bevylite UI Design System & Theme Tokens.
//!
//! Dark-first, futuristic, minimal palette inspired by Linear, Figma, and modern game editors.

#![allow(dead_code)]

use bevy::prelude::*;

// --- Background Surfaces ---
/// Deep obsidian canvas background
pub const BG_SHELL: Color = Color::srgb(0.035, 0.04, 0.055);
/// Surface for docked panels (Hierarchy, Inspector, Asset Browser)
pub const BG_PANEL: Color = Color::srgb(0.055, 0.065, 0.095);
/// Surface for cards, controls, and active containers
pub const BG_CARD: Color = Color::srgb(0.08, 0.095, 0.14);
/// Surface for hovered buttons and items
pub const BG_HOVER: Color = Color::srgb(0.12, 0.145, 0.21);
/// Surface for input fields and numeric displays
pub const BG_INPUT: Color = Color::srgb(0.06, 0.07, 0.105);

// --- Borders & Dividers ---
/// Ultra-thin subtle border for quiet visual hierarchy
pub const BORDER_SUBTLE: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);
/// Slightly brighter border for cards and containers
pub const BORDER_CARD: Color = Color::srgba(1.0, 1.0, 1.0, 0.13);
/// Cyan border glow for active or selected items
pub const BORDER_ACCENT: Color = Color::srgba(0.0, 0.949, 0.996, 0.45);

// --- Brand & Status Accents ---
/// Bevylite signature electric cyan accent
pub const ACCENT_CYAN: Color = Color::srgb(0.0, 0.949, 0.996);
/// Dim cyan for subtle pill backgrounds
pub const ACCENT_CYAN_DIM: Color = Color::srgba(0.0, 0.949, 0.996, 0.15);
/// Emerald green for Play Mode and success indicators
pub const ACCENT_EMERALD: Color = Color::srgb(0.063, 0.725, 0.506);
pub const ACCENT_EMERALD_HOVER: Color = Color::srgb(0.08, 0.82, 0.58);
/// Amber for warnings
pub const ACCENT_AMBER: Color = Color::srgb(0.96, 0.62, 0.04);
/// Crimson for errors and delete operations
pub const ACCENT_CRIMSON: Color = Color::srgb(0.94, 0.25, 0.25);

// --- Coordinate Axis Colors (Industry Standard) ---
/// X Axis: Crimson Red
pub const AXIS_X: Color = Color::srgb(0.937, 0.267, 0.267);
/// Y Axis: Emerald Green
pub const AXIS_Y: Color = Color::srgb(0.063, 0.725, 0.506);
/// Z Axis: Sky Blue / Cobalt
pub const AXIS_Z: Color = Color::srgb(0.22, 0.741, 0.973);

// --- Typography ---
/// High-contrast white for primary text and titles
pub const TEXT_PRIMARY: Color = Color::srgb(0.97, 0.98, 1.0);
/// Muted slate for secondary labels and descriptions
pub const TEXT_SECONDARY: Color = Color::srgb(0.58, 0.64, 0.72);
/// Darker slate for shortcuts, badges, and disabled states
pub const TEXT_MUTED: Color = Color::srgb(0.38, 0.44, 0.52);

/// Loaded font assets with rich symbol & emoji coverage
#[derive(Resource, Clone, Debug)]
pub struct EditorFonts {
    pub symbol: Handle<Font>,
    pub emoji: Handle<Font>,
    pub lucide: Handle<Font>,
}

impl EditorFonts {
    /// Helper to create a TextFont with symbol font support
    pub fn font_size(&self, px: f32) -> TextFont {
        TextFont {
            font: self.symbol.clone().into(),
            font_size: bevy::text::FontSize::Px(px),
            ..default()
        }
    }
}

