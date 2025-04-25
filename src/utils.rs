use std::path::PathBuf;
use gtk::{glib}; // Removed prelude, Box, Label, Orientation
use gtk::{ApplicationWindow, AlertDialog};
use gtk::gio; // Added gio import

/// Get the path to the notes directory
pub fn get_notes_dir() -> PathBuf {
    let user_data_dir = glib::user_data_dir();
    user_data_dir.join("juswriteit/notes")
}

/// Show an error dialog using AlertDialog
pub fn show_error_dialog(parent: &ApplicationWindow, title: &str, message: &str) {
    let dialog = AlertDialog::builder()
        .modal(true)
        .message(message)
        .detail(title) // Use detail for the title-like text
        .build();

    // Pass the slice directly, without Some()
    dialog.set_buttons(&["OK"]);
    dialog.set_default_button(0); // Index of the "OK" button

    // Show the dialog (no response needed for simple error)
    dialog.show(Some(parent));
}

/// Schedule an auto-save operation with a delay
pub fn schedule_auto_save<F: Fn() + 'static>(delay_ms: u32, callback: F) -> glib::SourceId {
    glib::timeout_add_local(
        std::time::Duration::from_millis(delay_ms.into()),
        move || {
            callback();
            // Don't repeat
            glib::ControlFlow::Break
        }
    )
}

/// Create a confirmation dialog using AlertDialog
pub fn show_confirmation_dialog<F: Fn() + 'static + Clone>(
    parent: &ApplicationWindow,
    _title: &str, // Prefix with underscore to silence warning
    message: &str,
    details: &str,
    confirm_action: F) {

    let dialog = AlertDialog::builder()
        .modal(true)
        .message(details) // Main message goes here
        .detail(message) // Title-like message goes heree
        .build();

    // Pass the slice directly, without Some()
    dialog.set_buttons(&["Cancel", "Confirm"]);
    // Note: AlertDialog doesn't have a direct way to set button appearance like Dialog did.
    // The appearance might depend on the theme or button order/role conventions.
    // We'll rely on the button text ("Confirm") for clarity.
    dialog.set_default_button(0); // Default to Cancel (index 0)
    dialog.set_cancel_button(0); // Cancel is index 0

    // Use glib's clone macro
    use glib::clone;

    // Choose presents the dialog and calls the callback with the index of the pressed button
    dialog.choose(Some(parent), None::<&gio::Cancellable>, clone!(@strong confirm_action => move |response| {
        // response is Result<i32, glib::Error> where i32 is the button index
        if let Ok(index) = response {
            if index == 1 { // Index 1 corresponds to "Confirm"
                confirm_action();
            }
        }
    }));
}
