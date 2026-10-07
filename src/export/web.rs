//! WebAssembly (WASM) / Web exporter.

use std::fs;
use crate::export::{ExportContext, ExportResult, Exporter};

#[derive(Default)]
pub struct WebExporter;

impl Exporter for WebExporter {
    fn id(&self) -> &'static str {
        "web"
    }

    fn name(&self) -> &'static str {
        "Web (WebAssembly / HTML5)"
    }

    fn description(&self) -> &'static str {
        "Exports a WebAssembly bundle suitable for modern web browsers and itch.io"
    }

    fn export(&self, ctx: &ExportContext) -> Result<ExportResult, String> {
        let export_dir = ctx.output_dir.join("web");
        fs::create_dir_all(&export_dir).map_err(|e| format!("Failed to create web export directory: {e}"))?;

        let index_html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <title>{}</title>
    <style>
        body, html {{ margin: 0; padding: 0; width: 100%; height: 100%; overflow: hidden; background-color: #111; }}
        canvas {{ width: 100%; height: 100%; display: block; }}
    </style>
</head>
<body>
    <script type="module">
        import init from './{}.js';
        init();
    </script>
</body>
</html>"#,
            ctx.project_name,
            ctx.project_name.to_lowercase().replace(' ', "_")
        );

        fs::write(export_dir.join("index.html"), index_html)
            .map_err(|e| format!("Failed to create index.html: {e}"))?;

        Ok(ExportResult {
            primary_artifact: export_dir.clone(),
            message: format!("Web bundle scaffolding ready at {:?}. Use `trunk build` or `wasm-pack` to build.", export_dir),
        })
    }
}
