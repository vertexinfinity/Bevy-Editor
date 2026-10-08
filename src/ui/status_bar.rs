//! Bottom status bar showing scene info, export feedback, and active object context.

use bevy::{inspector::InspectorSelection, prelude::*};
use crate::export::ExportStatus;
use crate::scene::EditorInternal;
use crate::ui::theme::*;

#[derive(Component)]
pub struct StatusText;

#[derive(Component)]
pub struct ActiveContextChip;

#[derive(Component)]
pub struct FpsCounterLabel;

#[derive(Component)]
pub struct SystemAlertChip;

pub fn spawn_status_bar(mut commands: Commands) {
    let bar_root = commands
        .spawn((
            EditorInternal,
            Name::new("BottomStatusBar"),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                height: px(28),
                border: UiRect::top(px(1)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(12), px(0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(BG_SHELL),
            BorderColor::all(BORDER_SUBTLE),
        ))
        .id();

    // Left container: Status dot + helper text
    let left_container = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    // Cyan Status Indicator Dot
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Node {
            width: px(7),
            height: px(7),
            border_radius: radius_pill(),
            ..default()
        },
        BackgroundColor(ACCENT_CYAN),
        ChildOf(left_container),
    ));

    // Left status text
    commands.spawn((
        EditorInternal,
        StatusText,
        Text::new("Ready | Fly Cam: WASD + Hold RMB | Speed: Scroll Wheel"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(left_container),
    ));

    // Right container: System Alert chip + FPS badge + Active context badge + Engine info
    let right_container = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                padding: UiRect::right(px(8)),
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    // System Warning Alert Chip (Low RAM / Low Disk)
    let alert_chip = commands
        .spawn((
            EditorInternal,
            SystemAlertChip,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border_radius: radius_button(),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(ACCENT_AMBER),
            ChildOf(right_container),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("⚠️ System Warning"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(ACCENT_AMBER),
        ChildOf(alert_chip),
    ));

    // FPS diagnostic chip
    let fps_chip = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border_radius: radius_button(),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(right_container),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        FpsCounterLabel,
        Text::new("60 FPS"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(fps_chip),
    ));

    // Active Context Badge (bottom right)
    let chip = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(2)),
                border_radius: radius_button(),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(right_container),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        ActiveContextChip,
        Text::new("No Selection"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(chip),
    ));

    // Right engine info text
    commands.spawn((
        EditorInternal,
        Text::new("Bevy 0.20-dev | Rust 2024"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(right_container),
    ));
}

/// Helper function to get free disk space in GiB for a given directory path on Windows / Unix.
fn get_available_disk_gib(path: &std::path::Path) -> Option<f64> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let wide_path: Vec<u16> = OsStr::new(path.as_os_str())
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetDiskFreeSpaceExW(
                lpDirectoryName: *const u16,
                lpFreeBytesAvailableToCaller: *mut u64,
                lpTotalNumberOfBytes: *mut u64,
                lpTotalNumberOfFreeBytes: *mut u64,
            ) -> i32;
        }

        let mut free_bytes_caller = 0u64;
        let mut total_bytes = 0u64;
        let mut total_free = 0u64;

        let res = unsafe {
            GetDiskFreeSpaceExW(
                wide_path.as_ptr(),
                &mut free_bytes_caller,
                &mut total_bytes,
                &mut total_free,
            )
        };

        if res != 0 {
            Some(free_bytes_caller as f64 / (1024.0 * 1024.0 * 1024.0))
        } else {
            None
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

pub fn update_status_bar(
    time: Res<Time>,
    export_status: Res<ExportStatus>,
    selection: Res<InspectorSelection>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    name_query: Query<Option<&Name>>,
    mut text_query: Query<&mut Text, (With<StatusText>, Without<ActiveContextChip>, Without<FpsCounterLabel>)>,
    mut chip_query: Query<&mut Text, (With<ActiveContextChip>, Without<StatusText>, Without<FpsCounterLabel>)>,
    mut fps_query: Query<&mut Text, (With<FpsCounterLabel>, Without<StatusText>, Without<ActiveContextChip>)>,
    mut alert_query: Query<(&mut Node, &mut BorderColor, &Children), With<SystemAlertChip>>,
    mut alert_text_query: Query<&mut Text, (Without<StatusText>, Without<ActiveContextChip>, Without<FpsCounterLabel>)>,
    mut check_timer: Local<f32>,
    mut cached_warning: Local<Option<(String, bool)>>, // (Message, is_critical)
) {
    // Smooth FPS calculation
    if let Ok(mut fps_text) = fps_query.single_mut() {
        let delta = time.delta_secs();
        if delta > 0.0001 {
            let fps = (1.0 / delta).round() as u32;
            fps_text.0 = format!("{fps} FPS");
        }
    }

    // Periodic system resource checks (every 3 seconds)
    *check_timer += time.delta_secs();
    if *check_timer >= 3.0 {
        *check_timer = 0.0;

        let mut warning = None;

        // Check RAM usage from DiagnosticsStore
        if let Some(diag) = diagnostics.get(&bevy::diagnostic::SystemInformationDiagnosticsPlugin::SYSTEM_MEM_USAGE) {
            if let Some(mem_pct) = diag.smoothed() {
                if mem_pct >= 90.0 {
                    warning = Some((format!("⚠️ Critically Low RAM ({mem_pct:.0}%)"), true));
                } else if mem_pct >= 82.0 {
                    warning = Some((format!("⚠️ Low RAM ({mem_pct:.0}%)"), false));
                }
            }
        }

        // Check Disk space on active working directory
        if warning.is_none() {
            if let Ok(cur_dir) = std::env::current_dir() {
                if let Some(free_gib) = get_available_disk_gib(&cur_dir) {
                    if free_gib < 2.0 {
                        warning = Some((format!("⚠️ Low Disk Space ({free_gib:.1} GB)"), true));
                    } else if free_gib < 5.0 {
                        warning = Some((format!("⚠️ Low Disk ({free_gib:.1} GB)"), false));
                    }
                }
            }
        }

        *cached_warning = warning;
    }

    // Update Alert Chip UI
    if let Ok((mut alert_node, mut alert_border, children)) = alert_query.single_mut() {
        if let Some((msg, is_crit)) = cached_warning.as_ref() {
            alert_node.display = Display::Flex;
            *alert_border = BorderColor::all(if *is_crit { ACCENT_CRIMSON } else { ACCENT_AMBER });
            for child in children.iter() {
                if let Ok(mut text) = alert_text_query.get_mut(child) {
                    text.0 = msg.clone();
                }
            }
        } else {
            alert_node.display = Display::None;
        }
    }

    if export_status.is_changed() {
        if let Ok(mut text) = text_query.single_mut() {
            match export_status.as_ref() {
                ExportStatus::Idle => {
                    text.0 = "Ready | Fly Cam: WASD + Hold RMB | Speed: Scroll Wheel".to_string();
                }
                ExportStatus::InProgress { target, progress } => {
                    text.0 = format!("Exporting {target}: {progress}");
                }
                ExportStatus::Success { target, message, .. } => {
                    text.0 = format!("Success [{target}]: {message}");
                }
                ExportStatus::Failed { target, error } => {
                    let first_line = error.lines().find(|l| !l.trim().is_empty()).unwrap_or("build error");
                    let short_err: String = first_line.chars().take(90).collect();
                    text.0 = format!("Error [{target}]: {short_err}");
                }
            }
        }
    }

    if selection.is_changed() {
        if let Ok(mut chip) = chip_query.single_mut() {
            if let Some(entity) = selection.0 {
                let name = name_query
                    .get(entity)
                    .ok()
                    .flatten()
                    .map(|n| n.as_str())
                    .unwrap_or("Object");
                chip.0 = format!("Active: {name} (#{entity})");
            } else {
                chip.0 = "No Selection".to_string();
            }
        }
    }
}

