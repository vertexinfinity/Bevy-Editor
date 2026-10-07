//! iOS Exporter supporting Xcode project scaffolding.

use std::fs;
use crate::export::{ExportContext, ExportResult, Exporter};

#[derive(Default)]
pub struct IosExporter;

impl Exporter for IosExporter {
    fn id(&self) -> &'static str {
        "ios"
    }

    fn name(&self) -> &'static str {
        "iOS (Xcode Project / IPA)"
    }

    fn description(&self) -> &'static str {
        "Scaffolds an iOS Xcode project for device deployment and App Store submission"
    }

    fn export(&self, ctx: &ExportContext) -> Result<ExportResult, String> {
        let export_dir = ctx.output_dir.join("ios");
        fs::create_dir_all(&export_dir).map_err(|e| format!("Failed to create ios export directory: {e}"))?;

        let info_plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>{}</string>
    <key>CFBundleIdentifier</key>
    <string>com.example.{}</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>UIRequiresFullScreen</key>
    <true/>
</dict>
</plist>"#,
            ctx.project_name,
            ctx.project_name.to_lowercase().replace(' ', "")
        );

        fs::write(export_dir.join("Info.plist"), info_plist)
            .map_err(|e| format!("Failed to write Info.plist: {e}"))?;

        Ok(ExportResult {
            primary_artifact: export_dir.clone(),
            message: format!("iOS Xcode configuration created at {:?}", export_dir),
        })
    }
}
