use gtk::prelude::*; // Import common GTK traits
use gtk::{glib, Application, ApplicationWindow, Paned, Orientation, Label, 
          ListBox, ScrolledWindow, Box}; // Added ListBox, ScrolledWindow, Box
use std::fs; // For directory creation and listing
use std::path::PathBuf; // For path manipulation

// Application ID (used by the system to identify the app)
// Follows reverse domain name notation
const APP_ID: &str = "com.example.juswriteit"; // Replace example.com with your domain or username

// Helper function to get the path to the notes directory
fn get_notes_dir() -> PathBuf {
    // glib::user_data_dir() returns a PathBuf directly
    let user_data_dir = glib::user_data_dir();
    
    // Join the path
    user_data_dir.join("juswriteit/notes")
}

// Function to ensure the notes directory exists
fn ensure_notes_dir_exists() -> Result<PathBuf, String> {
    let notes_dir = get_notes_dir();
    if !notes_dir.exists() {
        println!("Notes directory not found, creating at: {:?}", notes_dir);
        fs::create_dir_all(&notes_dir)
            .map_err(|e| format!("Failed to create notes directory {:?}: {}", notes_dir, e))?;
    }
    Ok(notes_dir)
}

fn main() -> glib::ExitCode {
    // Ensure the notes directory exists before starting the app
    if let Err(err) = ensure_notes_dir_exists() {
        eprintln!("Error initializing notes directory: {}", err);
        // Optionally show a graphical error dialog here later
        return glib::ExitCode::FAILURE; // Indicate failure
    }


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

    // Create a ListBox for the notes list
    let list_box = ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single) // Allow only one selection
        .build();

    // Create a ScrolledWindow to contain the ListBox with scrolling
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never) // Disable horizontal scrolling
        .min_content_width(200) // Minimum width
        .child(&list_box) // Add ListBox to ScrolledWindow
        .build();

    // Create a Box to hold the ScrolledWindow (for potential header/footer later)
    let left_pane = Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    
    left_pane.append(&scrolled_window);

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

    // Populate the notes list
    populate_notes_list(&list_box);

    // Present the window to the user
    window.present();
}

// Function to populate the ListBox with notes
fn populate_notes_list(list_box: &ListBox) {
    // Get the notes directory
    let notes_dir = get_notes_dir();
    
    // Read the directory entries
    match fs::read_dir(&notes_dir) {
        Ok(entries) => {
            let mut found_notes = false;
            
            // Process each entry
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    
                    // Check if it's a .md file
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                        found_notes = true;
                        
                        // Get the filename without the .md extension as the note title
                        if let Some(filename) = path.file_stem() {
                            if let Some(title) = filename.to_str() {
                                // Create a label with the note title
                                let label = Label::builder()
                                    .label(title)
                                    .xalign(0.0) // Left-align text
                                    .margin_start(5)
                                    .margin_end(5)
                                    .margin_top(5)
                                    .margin_bottom(5)
                                    .build();
                                
                                // Create a ListBoxRow and add the label to it
                                let row = gtk::ListBoxRow::new();
                                row.set_child(Some(&label));
                                
                                // Add the row to the ListBox
                                list_box.append(&row);
                            }
                        }
                    }
                }
            }
            
            // If no notes were found, show a placeholder message
            if !found_notes {
                let label = Label::builder()
                    .label("No notes yet. Create one!")
                    .xalign(0.0)
                    .margin_start(5)
                    .margin_end(5)
                    .margin_top(5)
                    .margin_bottom(5)
                    .build();
                
                let row = gtk::ListBoxRow::new();
                row.set_child(Some(&label));
                row.set_sensitive(false); // Make it non-selectable
                list_box.append(&row);
            }
        },
        Err(e) => {
            eprintln!("Error reading notes directory: {}", e);
            // Add an error message to the list
            let label = Label::builder()
                .label("Error loading notes")
                .xalign(0.0)
                .margin_start(5)
                .margin_end(5)
                .margin_top(5)
                .margin_bottom(5)
                .build();
            
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&label));
            list_box.append(&row);
        }
    }
}
