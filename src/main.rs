use gtk::prelude::*; // Import common GTK traits
use gtk::{glib, Application, ApplicationWindow};

// Application ID (used by the system to identify the app)
// Follows reverse domain name notation
const APP_ID: &str = "com.example.juswriteit"; // Replace example.com with your domain or username

fn main() -> glib::ExitCode {
    // Register resources - Placeholder for future icons, CSS, etc.
    // gio::resources_register_include!("compiled.gresource")
    //     .expect("Failed to register resources.");

    // Create a new GTK application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to the "activate" signal to build the UI when the app starts
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

// Function to build the user interface
fn build_ui(app: &Application) {
    // Create the main application window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("juswriteit") // Initial window title
        .default_width(800)  // Sensible default size
        .default_height(600)
        // .icon_name("org.gtk.TextEditor") // Example: Use a standard icon for now
        .build();

    // TODO: Add widgets like Paned, ListBox, TextView here later

    // Present the window to the user
    window.present();
}
