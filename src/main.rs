mod note;
mod ui;
mod utils;

use gtk::prelude::*;
use gtk::{glib, Application};
use std::fs;

use crate::utils::get_notes_dir;
use crate::ui::build_ui;  // Add this import

// Application ID (used by the system to identify the app)
const APP_ID: &str = "dev.penscript.Penscript";

fn main() -> glib::ExitCode {
    // Ensure the notes directory exists before starting the app
    if let Err(err) = ensure_notes_dir_exists() {
        eprintln!("Error initializing notes directory: {}", err);
        return glib::ExitCode::FAILURE;
    }

    // Create a new GTK application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to the "activate" signal to build the UI when the app starts
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

// Function to ensure the notes directory exists
fn ensure_notes_dir_exists() -> Result<(), String> {
    let notes_dir = get_notes_dir();
    if !notes_dir.exists() {
        println!("Notes directory not found, creating at: {:?}", notes_dir);
        fs::create_dir_all(&notes_dir)
            .map_err(|e| format!("Failed to create notes directory {:?}: {}", notes_dir, e))?;
    }
    Ok(())
}
