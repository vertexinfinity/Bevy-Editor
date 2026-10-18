//! Bevylite UI Design System & Theme Tokens.
//!
//! Dark-first, futuristic, minimal palette inspired by Linear, Figma, and modern game editors.

#![allow(dead_code)]

use bevy::prelude::*;

// --- Background Surfaces ---
/// Deep obsidian canvas background
pub const BG_SHELL: Color = Color::srgb(0.063, 0.075, 0.094);
/// Surface for docked panels (Hierarchy, Inspector, Asset Browser)
pub const BG_PANEL: Color = Color::srgb(0.086, 0.106, 0.133);
/// Surface for cards, controls, and active containers
pub const BG_CARD: Color = Color::srgb(0.102, 0.122, 0.149);
/// Surface for hovered buttons and items
pub const BG_HOVER: Color = Color::srgb(0.157, 0.188, 0.224);
/// Surface for input fields and numeric displays
pub const BG_INPUT: Color = Color::srgb(0.125, 0.149, 0.188);

// --- Borders & Dividers ---
/// Ultra-thin subtle border for quiet visual hierarchy
pub const BORDER_SUBTLE: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);
/// Slightly brighter border for cards and containers
pub const BORDER_CARD: Color = Color::srgba(1.0, 1.0, 1.0, 0.13);
/// Cyan border glow for active or selected items
pub const BORDER_ACCENT: Color = Color::srgba(0.271, 0.773, 0.859, 0.65);

// --- Brand & Status Accents ---
/// Bevylite signature electric cyan accent
pub const ACCENT_CYAN: Color = Color::srgb(0.271, 0.773, 0.859);
/// Dim cyan for subtle pill backgrounds
pub const ACCENT_CYAN_DIM: Color = Color::srgba(0.271, 0.773, 0.859, 0.13);
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
pub const TEXT_PRIMARY: Color = Color::srgb(0.898, 0.914, 0.941);
/// Muted slate for secondary labels and descriptions
pub const TEXT_SECONDARY: Color = Color::srgb(0.573, 0.608, 0.667);
/// Darker slate for shortcuts, badges, and disabled states
pub const TEXT_MUTED: Color = Color::srgb(0.38, 0.44, 0.52);

// --- Design System Standards (Linear / VS Code / Figma) ---
#[inline]
pub fn radius_panel() -> BorderRadius {
    BorderRadius::all(Val::Px(6.0))
}

#[inline]
pub fn radius_card() -> BorderRadius {
    BorderRadius::all(Val::Px(5.0))
}

#[inline]
pub fn radius_button() -> BorderRadius {
    BorderRadius::all(Val::Px(4.0))
}

#[inline]
pub fn radius_pill() -> BorderRadius {
    BorderRadius::all(Val::Px(3.0))
}

// Spacing Scale
pub const SPACE_MICRO: Val = Val::Px(4.0);
pub const SPACE_COMPACT: Val = Val::Px(8.0);
pub const SPACE_STANDARD: Val = Val::Px(12.0);
pub const SPACE_SPACIOUS: Val = Val::Px(16.0);
pub const SPACE_SECTION: Val = Val::Px(24.0);

// Control Dimensions
pub const HEIGHT_TOOLBAR: Val = Val::Px(34.0);
pub const HEIGHT_PANEL_HEADER: Val = Val::Px(32.0);
pub const HEIGHT_BTN_STANDARD: Val = Val::Px(26.0);
pub const HEIGHT_BTN_COMPACT: Val = Val::Px(22.0);
pub const HEIGHT_STATUS_BAR: Val = Val::Px(26.0);

// Font Size Scale
pub const FONT_SIZE_SMALL: f32 = 11.5;
pub const FONT_SIZE_NORMAL: f32 = 13.0;
pub const FONT_SIZE_SECTION: f32 = 14.0;
pub const FONT_SIZE_TITLE: f32 = 22.0;

/// Loaded font assets with rich symbol & emoji coverage
#[derive(Resource, Clone, Debug)]
pub struct EditorFonts {
    pub symbol: Handle<Font>,
    pub emoji: Handle<Font>,
    pub lucide: Handle<Font>,
}

impl EditorFonts {
    /// Helper to create a TextFont with standard editor typography
    pub fn font_size(&self, px: f32) -> TextFont {
        TextFont {
            font: self.symbol.clone().into(),
            font_size: bevy::text::FontSize::Px(px),
            ..default()
        }
    }

    pub fn small(&self) -> TextFont {
        self.font_size(FONT_SIZE_SMALL)
    }

    pub fn normal(&self) -> TextFont {
        self.font_size(FONT_SIZE_NORMAL)
    }

    pub fn section(&self) -> TextFont {
        self.font_size(FONT_SIZE_SECTION)
    }

    pub fn title(&self) -> TextFont {
        self.font_size(FONT_SIZE_TITLE)
    }
}


