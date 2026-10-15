//! Bevylite: Unified 3D Visual Editor & Headless Engine Tooling for Bevy.
//!
//! USAGE:
//!     bevylite                    # Launches the full 3D visual editor window
//!     bevylite -c                 # Runs in headless console mode
//!     bevylite --headless [CMD]   # Runs headlessly without window creation
//!     bevylite <SUBCOMMAND>       # Executes CLI command directly (new, build, mcp, etc.)

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Default: No arguments passed -> launch visual 3D editor
    if args.len() <= 1 {
        bevy_editor_lib::run_editor();
        return;
    }

    match args[1].as_str() {
        // Explicit GUI flag
        "--gui" | "gui" => {
            bevy_editor_lib::run_editor();
        }
        // Open scene or project in visual editor
        "open" => {
            bevy_editor_lib::cli::handle_open(&args[2..]);
        }
        // Headless / Console flags or CLI subcommands
        _ => {
            bevy_editor_lib::cli::dispatch(&args[1..]);
        }
    }
}
