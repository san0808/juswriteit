use gtk::prelude::*; // Import common GTK traits
use gtk::{glib, Application, ApplicationWindow, Paned, Orientation, Label}; // Added Paned, Orientation, Label

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

    // Create a Paned widget (Horizontal orientation for left/right split)
    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .wide_handle(true) // Makes the separator easier to grab
        .build();

    // Placeholder for the left pane (Note List)
    let left_pane = Label::builder()
        .label("Note List Area")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();
    // Make the left pane shrinkable but not expandable by default
    left_pane.set_size_request(200, -1); // Request a minimum width

    // Placeholder for the right pane (Editor)
    let right_pane = Label::builder()
        .label("Editor Area")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .hexpand(true) // Allow the right pane to expand
        .vexpand(true)
        .build();

    // Add the panes to the Paned widget
    paned.set_start_child(Some(&left_pane));
    paned.set_end_child(Some(&right_pane));

    // Set the initial position of the divider (e.g., 250 pixels from the left)
    paned.set_position(250);

    // Set the Paned widget as the child of the window
    window.set_child(Some(&paned));

    // Present the window to the user
    window.present();
}
