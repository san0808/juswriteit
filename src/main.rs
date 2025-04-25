use gtk::prelude::*; // Import common GTK traits
use gtk::{glib, Application, ApplicationWindow, Paned, Orientation, Label,
          ListBox, ScrolledWindow, Box, TextView, HeaderBar, Button};
use glib::clone; // Import the clone macro from glib
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::cell::RefCell;
use std::rc::Rc;
use chrono::Local; // For better date formatting

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
    use gtk::EventControllerKey; // Add import locally where needed

    // Create the main application window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("juswriteit") // Initial window title
        .default_width(800)  // Sensible default size
        .default_height(600)
        .build();

    // Create a HeaderBar
    let header_bar = HeaderBar::builder()
        .show_title_buttons(true) // Show minimize/maximize/close buttons
        .title_widget(&Label::new(Some("juswriteit")))
        .build();
    
    // Create buttons for the header bar
    let new_note_button = Button::builder()
        .label("New Note")
        .tooltip_text("Create a new note")
        .build();
    
    let delete_note_button = Button::builder()
        .label("Delete")
        .tooltip_text("Delete current note")
        .sensitive(false) // Initially disabled until a note is selected
        .build();
    
    // Add buttons to the HeaderBar
    header_bar.pack_start(&new_note_button);
    header_bar.pack_end(&delete_note_button);
    
    // Set the HeaderBar as the window's titlebar
    window.set_titlebar(Some(&header_bar));

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
        .build();
    
    scrolled_window.set_child(Some(&list_box));

    // Create a Box to hold the ScrolledWindow (for potential header/footer later)
    let left_pane = Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    
    left_pane.append(&scrolled_window);

    // Create the editor (TextView) for the right pane
    let text_view = TextView::builder()
        .wrap_mode(gtk::WrapMode::Word) // Enable word wrapping
        .monospace(false) // Use proportional font (not monospace) for better readability
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    text_view.set_editable(true);

    // Create a ScrolledWindow to contain the TextView with scrolling
    let editor_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never) // Disable horizontal scrolling (due to word wrap)
        .hexpand(true) // Allow to expand horizontally
        .vexpand(true) // Allow to expand vertically
        .build();
    
    editor_scrolled_window.set_child(Some(&text_view));

    // Create a shared variable to track the currently selected note path and name
    let current_note: Rc<RefCell<Option<(PathBuf, String)>>> = Rc::new(RefCell::new(None));
    
    // Enable/disable delete button based on selection
    let delete_button_ref = delete_note_button.clone();
    let current_note_for_button = current_note.clone();
    let update_ui_for_selection = move || {
        let has_selection = current_note_for_button.borrow().is_some();
        delete_button_ref.set_sensitive(has_selection);
    };

    // Clone for row selection handler
    let text_view_for_loading = text_view.clone();
    let current_note_for_loading = current_note.clone();
    let update_ui_for_selection_on_load = update_ui_for_selection.clone();
    let window_for_loading = window.clone(); // Clone window *before* moving it

    // Connect the row-selected signal to load note content
    list_box.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            // Get the row index
            let index = row.index();
            
            // Skip if it's a placeholder or error row 
            if index < 0 {
                *current_note_for_loading.borrow_mut() = None;
                update_ui_for_selection_on_load();
                return;
            }
            
            // Get the note title from the row's child (Label)
            if let Some(label) = row.child().and_then(|w| w.downcast::<Label>().ok()) {
                let title = label.label();
                let notes_dir = get_notes_dir();
                let file_path = notes_dir.join(format!("{}.md", title));
                
                // Update the current note path and title
                *current_note_for_loading.borrow_mut() = Some((file_path.clone(), title.to_string()));
                
                // Load the content into the TextView
                match load_note_content(&text_view_for_loading, &file_path) {
                    Ok(_) => {
                        // Use the cloned window here
                        window_for_loading.set_title(Some(&format!("{} - juswriteit", title)));
                    },
                    Err(e) => {
                        eprintln!("Error loading note content: {}", e);
                        // Clear the buffer on error
                        text_view_for_loading.buffer().set_text("");
                        // Use the cloned window here
                        window_for_loading.set_title(Some("juswriteit"));
                    }
                }
            }
        } else {
            // No row selected, clear the TextView
            text_view_for_loading.buffer().set_text("");
            
            // Clear the current note
            *current_note_for_loading.borrow_mut() = None;
            
            // Reset window title
            window_for_loading.set_title(Some("juswriteit"));
        }
        
        // Update UI state based on selection
        update_ui_for_selection_on_load();
    });

    // Add keyboard shortcut handling (Ctrl+S for save)
    let key_controller = EventControllerKey::new();

    // Clone what we need for the closure
    let text_view_for_save = text_view.clone();
    let current_note_for_save = current_note.clone();
    let window_for_save = window.clone(); // Clone the original window here

    key_controller.connect_key_pressed(clone!(@strong window_for_save, @strong text_view_for_save, @strong current_note_for_save => move |_, key, _keycode, state| {
        // Check for Ctrl+S - using gtk constants
        if key == gtk::gdk::Key::s && state.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            if let Some((file_path, _)) = current_note_for_save.borrow().clone() {
                match save_note_content(&text_view_for_save, &file_path) {
                    Ok(_) => {
                        // Show temporary save indicator in window title
                        let current_title = window_for_save.title().unwrap_or_else(|| "juswriteit".into());
                        let title_parts: Vec<&str> = current_title.split(" - ").collect();
                        let base_title = if title_parts.len() > 1 {
                            format!("{} - juswriteit", title_parts[0])
                        } else {
                            current_title.to_string()
                        };
                        
                        window_for_save.set_title(Some(&format!("{} (Saved)", base_title)));
                        
                        // Reset title after 2 seconds
                        let window_ref = window_for_save.clone();
                        let base_title_clone = base_title.clone();
                        glib::timeout_add_seconds_local(2, move || {
                            window_ref.set_title(Some(&base_title_clone));
                            // Fix: Use glib::ControlFlow instead of glib::Continue
                            glib::ControlFlow::Break
                        });
                    },
                    Err(e) => {
                        eprintln!("Error saving note: {}", e);
                        show_error_dialog(&window_for_save, "Save Error", &format!("Failed to save note: {}", e));
                    }
                }
            } else {
                eprintln!("No note is currently selected to save");
                // Could add a create new note prompt here
            }
            
            // Return true to indicate we handled the key press
            return glib::Propagation::Stop;
        }
        
        // Let other handlers process the key press
        glib::Propagation::Proceed
    }));
    // Add the controller to the window
    window.add_controller(key_controller);
    
    // Clone references for the "New Note" button
    let list_box_for_new = list_box.clone();
    let notes_dir_for_new = get_notes_dir();
    let current_note_for_new = current_note.clone();
    let text_view_for_new = text_view.clone();
    let window_for_new = window.clone(); // Clone the original window here
    
    // Connect the "clicked" signal to create a new note
    new_note_button.connect_clicked(clone!(@strong window_for_new, @strong list_box_for_new, @strong notes_dir_for_new, @strong current_note_for_new, @strong text_view_for_new => move |_| {
        // Generate a new note with a readable timestamp
        let now: chrono::DateTime<Local> = Local::now();
        let formatted_date = now.format("%Y-%m-%d").to_string();
        
        // Create a new note with an incrementing number if needed
        let mut note_number = 1;
        let mut note_title = format!("Note {}", formatted_date);
        let mut file_path = notes_dir_for_new.join(format!("{}.md", note_title));
        
        // Check if file already exists, increment until we find a unique name
        while file_path.exists() {
            note_number += 1;
            note_title = format!("Note {} ({})", formatted_date, note_number);
            file_path = notes_dir_for_new.join(format!("{}.md", note_title));
        }
        
        // Create an empty file
        match File::create(&file_path) {
            Ok(_) => {
                println!("Created new note: {:?}", file_path);
                
                // Clear the editor
                text_view_for_new.buffer().set_text("");
                
                // Update the current note
                *current_note_for_new.borrow_mut() = Some((file_path.clone(), note_title.clone()));
                
                // Update window title
                window_for_new.set_title(Some(&format!("{} - juswriteit", note_title)));
                
                // Refresh the list box to show the new note
                refresh_notes_list(&list_box_for_new);
                
                // Find and select the newly created note
                let note_file_name = format!("{}.md", note_title);
                select_note_by_filename(&list_box_for_new, &note_file_name);
            },
            Err(e) => {
                eprintln!("Error creating new note: {}", e);
                show_error_dialog(&window_for_new, "Create Error", &format!("Failed to create new note: {}", e));
            }
        }
    }));
    
    // Connect the delete button
    let list_box_for_delete = list_box.clone();
    let current_note_for_delete = current_note.clone();
    let window_for_delete = window.clone(); // Clone the original window here
    let text_view_for_delete = text_view.clone();
    
    delete_note_button.connect_clicked(clone!(@strong window_for_delete, @strong list_box_for_delete, @strong current_note_for_delete, @strong text_view_for_delete => move |_| {
        if let Some((file_path, title)) = current_note_for_delete.borrow().clone() {
            // Create a simple confirmation dialog
            let dialog = gtk::Dialog::builder()
                .title("Confirm Deletion")
                .transient_for(&window_for_delete)
                .modal(true)
                .build();
            
            // Add message
            let content_area = dialog.content_area();
            let message_box = gtk::Box::builder()
                .orientation(Orientation::Vertical)
                .spacing(10)
                .margin_start(20)
                .margin_end(20)
                .margin_top(20)
                .margin_bottom(20)
                .build();
            
            let title_label = gtk::Label::builder()
                .label(&format!("Delete note \"{}\"?", title))
                .xalign(0.0)
                .build();
            title_label.add_css_class("title-3");
            
            let detail_label = gtk::Label::builder()
                .label("This action cannot be undone.")
                .xalign(0.0)
                .build();
            
            message_box.append(&title_label);
            message_box.append(&detail_label);
            content_area.append(&message_box);
            
            // Add Cancel button
            dialog.add_button("Cancel", gtk::ResponseType::Cancel);
            
            // Add Delete button (destructive)
            let delete_button = dialog.add_button("Delete", gtk::ResponseType::Accept);
            delete_button.add_css_class("destructive-action");
            
            dialog.set_default_response(gtk::ResponseType::Cancel);
            
            // Handle response
            dialog.connect_response(clone!(@strong current_note_for_delete, @strong list_box_for_delete, 
                                          @strong text_view_for_delete, @strong window_for_delete,
                                          @strong file_path => move |dialog, response| {
                dialog.close();
                
                if response == gtk::ResponseType::Accept {
                    // User confirmed deletion
                    match fs::remove_file(&file_path) {
                        Ok(_) => {
                            println!("Deleted note: {:?}", file_path);
                            
                            // Clear the current note
                            *current_note_for_delete.borrow_mut() = None;
                            
                            // Clear the editor
                            text_view_for_delete.buffer().set_text("");
                            
                            // Reset window title
                            window_for_delete.set_title(Some("juswriteit"));
                            
                            // Refresh the list box
                            refresh_notes_list(&list_box_for_delete);
                        },
                        Err(e) => {
                            eprintln!("Error deleting note: {}", e);
                            show_error_dialog(&window_for_delete, "Delete Error", 
                                          &format!("Failed to delete note: {}", e));
                        }
                    }
                }
            }));
            
            dialog.present();
        }
    }));

    // Add the panes to the Paned widget
    paned.set_start_child(Some(&left_pane));
    paned.set_end_child(Some(&editor_scrolled_window));

    // Set the initial position of the divider
    paned.set_position(250);

    // Set the Paned widget as the child of the window
    window.set_child(Some(&paned));

    // Populate the notes list
    populate_notes_list(&list_box);

    // Present the window to the user
    window.present();
}

// Function to find and select a note by its filename
fn select_note_by_filename(list_box: &ListBox, filename: &str) {
    let mut row_index = 0;
    
    while let Some(row) = list_box.row_at_index(row_index) {
        if let Some(label) = row.child().and_then(|w| w.downcast::<Label>().ok()) {
            let title = label.label();
            let note_filename = format!("{}.md", title);
            
            if note_filename == filename {
                list_box.select_row(Some(&row));
                
                // Ensure the row is visible by scrolling to it
                row.grab_focus();
                return;
            }
        }
        
        row_index += 1;
    }
}

// Function to show an error dialog
fn show_error_dialog(parent: &ApplicationWindow, title: &str, message: &str) {
    // Create a simple dialog
    let dialog = gtk::Dialog::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .build();
    
    // Add message
    let content_area = dialog.content_area();
    let message_label = gtk::Label::builder()
        .label(message)
        .xalign(0.0)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .build();
    
    content_area.append(&message_label);
    
    // Add OK button
    dialog.add_button("OK", gtk::ResponseType::Ok);
    dialog.set_default_response(gtk::ResponseType::Ok);
    
    // Auto-close on response
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    
    dialog.present();
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

// Function to refresh the notes list (clear and repopulate)
fn refresh_notes_list(list_box: &ListBox) {
    // Remove all existing rows
    while let Some(row) = list_box.row_at_index(0) {
        list_box.remove(&row);
    }
    
    // Repopulate the list
    populate_notes_list(list_box);
}

// Function to load note content from a file into a TextView
fn load_note_content(text_view: &TextView, file_path: &PathBuf) -> Result<(), String> {
    // Get the TextView's buffer (this directly returns a TextBuffer, not an Option)
    let buffer = text_view.buffer();
    
    // Read file content
    let mut file = File::open(file_path)
        .map_err(|e| format!("Failed to open file {}: {}", file_path.display(), e))?;
    
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read file {}: {}", file_path.display(), e))?;
    
    // Set content to buffer
    buffer.set_text(&content);
    
    Ok(())
}

// Function to save note content from a TextView to a file
fn save_note_content(text_view: &TextView, file_path: &PathBuf) -> Result<(), String> {
    // Get the TextView's buffer
    let buffer = text_view.buffer();
    
    // Get the content from the buffer (start to end)
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    let content = buffer.text(&start, &end, true)
        .to_string(); // Convert GString to String directly
    
    // Write content to file
    let mut file = File::create(file_path)
        .map_err(|e| format!("Failed to create file {}: {}", file_path.display(), e))?;
    
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write to file {}: {}", file_path.display(), e))?;
    
    Ok(())
}
