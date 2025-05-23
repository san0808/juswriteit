use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Paned, Orientation, Label,
          ListBox, ScrolledWindow, Box, TextView, Button,
          EventControllerKey, CssProvider, Overlay, WindowHandle, WindowControls,
          SearchEntry};
use glib::{clone, Propagation};
use gtk::gdk::{Key, ModifierType};
use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use std::sync::Once;

use crate::note::Note;
use crate::utils::{show_error_dialog, show_confirmation_dialog, schedule_auto_save};

// Struct to handle active note state
// #[derive(Clone)] // Remove derive Clone
struct ActiveNote {
    path: PathBuf,
    title: String,
    has_changes: bool,
    auto_save_source_id: Option<glib::SourceId>,
    note: Note,
}

// Manual implementation of Clone for ActiveNote
impl Clone for ActiveNote {
    fn clone(&self) -> Self {
        ActiveNote {
            path: self.path.clone(),
            title: self.title.clone(),
            has_changes: self.has_changes,
            auto_save_source_id: None, // SourceId cannot be cloned, set to None
            note: self.note.clone(),
        }
    }
}

// Auto-save delay in milliseconds
const AUTO_SAVE_DELAY_MS: u32 = 2000; // 2 seconds

// Flag to indicate programmatic text changes
thread_local! {
    static PROGRAMMATIC_TEXT_CHANGE: RefCell<bool> = RefCell::new(false);
}

// Add app name constant
const APP_NAME: &str = "Penscript";
const INITIAL_WINDOW_WIDTH: i32 = 1000;
const INITIAL_WINDOW_HEIGHT: i32 = 700;
const INITIAL_SIDEBAR_WIDTH: i32 = 250; // Fixed width for a clean look

/// Build the user interface
pub fn build_ui(app: &Application) {
    // Load CSS for styling
    load_css();
    
    // Create the main application window - undecorated
    let window = ApplicationWindow::builder()
        .application(app)
        .title(APP_NAME)
        .default_width(INITIAL_WINDOW_WIDTH)
        .default_height(INITIAL_WINDOW_HEIGHT)
        .css_classes(vec!["dark-mode", "transition"])
        .decorated(false) // Make window frameless
        .build();

    // Main overlay for UI elements
    let main_overlay = Overlay::new();
    
    // Create window handle for drag operations
    let window_handle = WindowHandle::builder()
        .css_classes(vec!["window-handle"])
        .hexpand(true)
        .build();
    
    // Create window controls (minimize, maximize, close)
    let window_controls = WindowControls::builder()
        .side(gtk::PackType::End)
        .css_classes(vec!["window-controls"])
        .build();
    
    // Create a top bar container to hold window controls
    let top_bar = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(vec!["top-bar"])
        .build();
    
    top_bar.append(&window_handle);
    top_bar.append(&window_controls);
    
    // Create a Paned widget with horizontal orientation
    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .wide_handle(true) // Make handle easier to grab
        .position(INITIAL_SIDEBAR_WIDTH) // Set initial position
        .css_classes(vec!["main-pane"])
        .hexpand(true)
        .vexpand(true)
        .build();

    // --- Sidebar Setup ---
    // Update sidebar header styling
    let sidebar_header_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(vec!["sidebar-header-box"])
        .margin_bottom(6)
        .build();
        
    let notes_label = Label::builder()
        .label("NOTES")
        .xalign(0.0)
        .hexpand(true)
        .margin_start(12)
        .margin_top(6)
        .margin_bottom(6)
        .css_classes(vec!["sidebar-header"])
        .build();
    
    // Add "New Note" button with minimalist styling
    let new_note_button = Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("New Note")
        .css_classes(vec!["icon-only-button"])
        .margin_end(8)
        .build();
        
    sidebar_header_box.append(&notes_label);
    sidebar_header_box.append(&new_note_button);

    // ListBox and rest of sidebar setup
    let list_box = ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(vec!["notes-list"])
        .vexpand(true)
        .build();

    // Create a ScrolledWindow to contain the ListBox with scrolling
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .css_classes(vec!["sidebar-scroll"])
        .build();
    
    scrolled_window.set_child(Some(&list_box));

    // Create a Box to hold the sidebar components
    let left_pane = Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(vec!["sidebar"])
        .build();
    
    // Add the sidebar components
    left_pane.append(&sidebar_header_box); // Use the box containing label and button

    // Create SearchEntry for filtering notes
    let search_entry = SearchEntry::builder()
        .placeholder_text("Search notes...")
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .hexpand(true)
        .css_classes(vec!["search-entry"]) // For potential styling
        .build();
    
    left_pane.append(&search_entry); // Add search entry below header, above list
    left_pane.append(&scrolled_window);

    // --- Filtering Logic for SearchEntry ---
    let list_box_for_search = list_box.clone();
    search_entry.connect_search_changed(move |search_bar| {
        let query = search_bar.text().to_lowercase();
        let list_box = &list_box_for_search;

        let mut current_row_index = 0;
        while let Some(row) = list_box.row_at_index(current_row_index) {
            if row.is_sensitive() { // Only process actual note rows, not placeholders like "No notes yet"
                let title_label_opt = row.child() // row_outer_box
                    .and_then(|outer_box| outer_box.downcast::<Box>().ok()) 
                    .and_then(|hbox| hbox.first_child()) // row_content_box
                    .and_then(|content_box| content_box.downcast::<Box>().ok()) 
                    .and_then(|vbox| vbox.first_child()) // title_label
                    .and_then(|widget| widget.downcast::<Label>().ok());

                if let Some(title_label) = title_label_opt {
                    let title = title_label.label().to_lowercase();
                    if query.is_empty() || title.contains(&query) {
                        row.set_visible(true);
                    } else {
                        row.set_visible(false);
                    }
                } else {
                    // If title can't be extracted, make it visible by default if query is empty
                    row.set_visible(query.is_empty());
                }
            } else {
                 // For non-sensitive rows (like "No notes yet" placeholder), hide if there's a query
                row.set_visible(query.is_empty());
            }
            current_row_index += 1;
        }
    });

    // --- Right Pane (Editor Area) Setup ---
    let right_pane = Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(vec!["editor-area"])
        .hexpand(true)
        .vexpand(true)
        .build();
    
    // Create a text view for editing with proper margins and styling
    let text_view = TextView::builder()
        .wrap_mode(gtk::WrapMode::Word)
        .monospace(true) // Use monospace font
        .css_classes(vec!["editor"])
        .hexpand(true)
        .vexpand(true)
        .top_margin(100) // Add top margin for padding
        .bottom_margin(100)
        .left_margin(60)
        .right_margin(60)
        .build();
    
    // Configure buffer with some initial settings
    let buffer = text_view.buffer();
    buffer.set_text("");
    
    // Ensure text_view is editable
    text_view.set_editable(true);
    
    // Create a ScrolledWindow to contain the TextView with scrolling
    let editor_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .vexpand(true)
        .css_classes(vec!["editor-scroll"])
        .build();
    
    editor_scrolled_window.set_child(Some(&text_view));

    // --- Status Bar Setup ---
    // App logo (center)
    let app_logo = Label::builder()
        .label(APP_NAME)
        .css_classes(vec!["app-logo"])
        .halign(gtk::Align::Center)
        .hexpand(true)
        .margin_start(20)
        .margin_end(20)
        .build();
        
    // Status text (left side)
    let status_label = Label::builder()
        .label("Ready")
        .xalign(0.0)
        .hexpand(false)
        .margin_start(10)
        .css_classes(vec!["status-text"])
        .build();
        
    // Word count (far right)
    let word_count_label = Label::builder()
        .label("0 words")
        .xalign(1.0)
        .margin_start(10)
        .margin_end(10)
        .css_classes(vec!["word-count"])
        .build();

    // Modify the bottom bar to match editor color
    let bottom_bar = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(vec!["bottom-bar"])
        .margin_top(0)
        .margin_bottom(0)
        .margin_start(0)
        .margin_end(0)
        .spacing(8)
        .height_request(38)
        .build();
    
    // Add buttons to controls container
    let controls_container = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::End)
        .hexpand(false)
        .build();
    
    // Create keyboard shortcuts button - use icon-only styling
    let shortcuts_button = Button::builder()
        .icon_name("input-keyboard-symbolic")
        .tooltip_text("Keyboard Shortcuts (Ctrl+K)")
        .css_classes(vec!["icon-only-button"])
        .build();
    
    // Create sidebar toggle button - use icon-only styling
    let sidebar_toggle = Button::builder()
        .icon_name("view-sidebar-start-symbolic")
        .tooltip_text("Toggle Sidebar (Ctrl+B)")
        .css_classes(vec!["icon-only-button"])
        .build();
    
    // Create theme toggle button - use icon-only styling
    let theme_toggle_button = Button::builder()
        .icon_name("weather-clear-night-symbolic")
        .tooltip_text("Toggle Light/Dark Theme (Ctrl+T)")
        .css_classes(vec!["icon-only-button"])
        .build();
    
    // Create fullscreen toggle button - use icon-only styling
    let fullscreen_button = Button::builder()
        .icon_name("view-fullscreen-symbolic")
        .tooltip_text("Toggle Fullscreen Mode (F11)")
        .css_classes(vec!["icon-only-button"])
        .build();
    
    // Add buttons to controls container
    controls_container.append(&shortcuts_button);
    controls_container.append(&sidebar_toggle);
    controls_container.append(&theme_toggle_button);
    controls_container.append(&fullscreen_button);
    
    // Add all elements to the bottom bar in proper order
    bottom_bar.append(&status_label);
    bottom_bar.append(&app_logo);
    bottom_bar.append(&controls_container);
    bottom_bar.append(&word_count_label);
    
    // Add editor components to right pane
    right_pane.append(&editor_scrolled_window);

    // --- Main Layout Assembly ---
    // Create main vertical box to hold all components
    let main_box = Box::builder()
        .orientation(Orientation::Vertical)
        .hexpand(true)
        .vexpand(true)
        .build();
    
    // Add top bar (window controls)
    main_box.append(&top_bar);
    
    // Add paned container (main content)
    main_box.append(&paned);
    
    // Add unified bottom bar at the bottom
    main_box.append(&bottom_bar);
    
    // Set main box as the overlay child
    main_overlay.set_child(Some(&main_box));
    
    // Set the overlay as the window child
    window.set_child(Some(&main_overlay));

    // --- Button functionality ---
    // Fullscreen toggle functionality
    let window_for_fullscreen = window.clone();
    fullscreen_button.connect_clicked(move |button| {
        let is_fullscreen = window_for_fullscreen.is_fullscreen();
        if is_fullscreen {
            window_for_fullscreen.unfullscreen();
            window_for_fullscreen.remove_css_class("fullscreen-mode");
            button.set_icon_name("view-restore-symbolic");
        } else {
            window_for_fullscreen.fullscreen();
            window_for_fullscreen.add_css_class("fullscreen-mode");
            button.set_icon_name("view-fullscreen-symbolic");
        }
    });
    
    // Theme toggle functionality
    let window_for_theme = window.clone();
    let bottom_bar_for_theme = bottom_bar.clone();
    let top_bar_for_theme = top_bar.clone();
    let left_pane_for_theme = left_pane.clone();
    let right_pane_for_theme = right_pane.clone(); // Add right pane
    let text_view_for_theme = text_view.clone(); // Add text view
    let editor_scrolled_window_for_theme = editor_scrolled_window.clone(); // Add scrolled window
    let controls_container_for_theme = controls_container.clone();
    
    theme_toggle_button.connect_clicked(move |button| {
        if window_for_theme.has_css_class("dark-mode") {
            // Switch to light mode
            window_for_theme.remove_css_class("dark-mode");
            window_for_theme.add_css_class("light-mode");
            
            // Apply light mode to all major UI components
            bottom_bar_for_theme.add_css_class("light-mode");
            top_bar_for_theme.add_css_class("light-mode");
            left_pane_for_theme.add_css_class("light-mode");
            right_pane_for_theme.add_css_class("light-mode"); // Add light mode to right pane
            text_view_for_theme.add_css_class("light-mode"); // Add light mode to text view
            editor_scrolled_window_for_theme.add_css_class("light-mode"); // Add light mode to editor scrolled window
            controls_container_for_theme.add_css_class("light-mode");
            
            button.set_icon_name("weather-clear-symbolic");
        } else {
            // Switch to dark mode
            window_for_theme.remove_css_class("light-mode");
            window_for_theme.add_css_class("dark-mode");
            
            // Remove light mode from all major UI components
            bottom_bar_for_theme.remove_css_class("light-mode");
            top_bar_for_theme.remove_css_class("light-mode");
            left_pane_for_theme.remove_css_class("light-mode");
            right_pane_for_theme.remove_css_class("light-mode"); // Remove light mode from right pane
            text_view_for_theme.remove_css_class("light-mode"); // Remove light mode from text view
            editor_scrolled_window_for_theme.remove_css_class("light-mode"); // Remove light mode from editor scrolled window
            controls_container_for_theme.remove_css_class("light-mode");
            
            button.set_icon_name("weather-clear-night-symbolic");
        }
    });
    
    // Sidebar toggle functionality
    let window_for_sidebar = window.clone();
    let left_pane_for_sidebar = left_pane.clone();
    let paned_for_sidebar = paned.clone();
    sidebar_toggle.connect_clicked(move |_| {
        if window_for_sidebar.has_css_class("sidebar-hidden") {
            window_for_sidebar.remove_css_class("sidebar-hidden");
            left_pane_for_sidebar.set_visible(true);
            // Restore to a reasonable position, could be dynamic based on current window width
            // For simplicity, using the initial calculated width
            paned_for_sidebar.set_position(INITIAL_SIDEBAR_WIDTH); 
        } else {
            window_for_sidebar.add_css_class("sidebar-hidden");
            paned_for_sidebar.set_position(0); // Hide by moving divider to the edge
        }
    });
    
    // Keyboard shortcuts dialog
    let window_for_shortcuts = window.clone();
    shortcuts_button.connect_clicked(move |_| {
        show_shortcuts_dialog(&window_for_shortcuts);
    });

    // --- Active Note Logic ---
    let active_note: Rc<RefCell<Option<ActiveNote>>> = Rc::new(RefCell::new(None));
    
    // --- Row Selection Logic ---
    // Clone variables needed for the closure
    let active_note_for_select = active_note.clone();
    let text_view_for_select = text_view.clone();
    let window_for_select = window.clone();
    let status_label_for_select = status_label.clone();
    let word_count_label_for_select = word_count_label.clone();

    // --- Fix the selection handler ---
    list_box.connect_row_selected(move |_listbox, row_opt| {
        // Synchronously save the currently active note if it has changes
        {
            let mut active_opt = active_note_for_select.borrow_mut();
            if let Some(active) = active_opt.as_mut() {
                if active.has_changes {
                    // Cancel any pending auto-save timer first
                    if let Some(source_id) = active.auto_save_source_id.take() {
                        let _ = source_id.remove();
                    }
                    // Attempt to save synchronously
                    match active.note.save() {
                        Ok(_) => {
                            active.has_changes = false;
                            status_label_for_select.set_text("Saved"); // Give feedback

                            // Refresh the list to update preview/timestamp
                            let list_box_for_refresh = _listbox.clone(); // _listbox is the first arg to connect_row_selected
                            let active_note_for_refresh = active_note_for_select.clone();
                            let window_for_refresh = window_for_select.clone();
                            let status_label_for_refresh = status_label_for_select.clone();
                            let word_count_label_for_refresh = word_count_label_for_select.clone();
                            let text_view_for_refresh = text_view_for_select.clone();
                            let _original_title_for_reselect = active.title.clone(); // THIS LINE: Ensure variable is _original_title_for_reselect

                            refresh_note_list(&list_box_for_refresh, &active_note_for_refresh, &window_for_refresh, &status_label_for_refresh, &word_count_label_for_refresh, &text_view_for_refresh);
                            select_note_by_title(&list_box_for_refresh, &_original_title_for_reselect); // Ensure uses non-prefixed

                            let status_label_clone = status_label_for_select.clone();
                            glib::timeout_add_seconds_local(2, move || { // Revert status after a bit
                                if status_label_clone.text() == "Saved" {
                                    status_label_clone.set_text("Ready");
                                }
                                glib::ControlFlow::Break
                            });
                        }
                        Err(e) => {
                            eprintln!("Error saving note {} before switching: {}", active.title, e);
                            // Show an error to the user. The 'window_for_select' is cloned earlier for this handler.
                            show_error_dialog(&window_for_select, "Save Error", &format!("Failed to save changes to note '{}' before switching: {}. Your changes were NOT saved. Please try again or check permissions/disk space.", active.title, e));
                            // Note: active.has_changes remains true, as the save failed.
                            return; // CRITICAL: Do not proceed with note switch if save failed
                        }
                    }
                } else {
                    // No changes, but if there's an auto-save timer for some reason, cancel it.
                    if let Some(source_id) = active.auto_save_source_id.take() {
                        let _ = source_id.remove();
                    }
                }
            }
        }
        // The original block that only cancelled the timer is now effectively covered by the logic above.

        if let Some(row) = row_opt {
            // Get the note title from the row's child structure
            let title = row.child()
                .and_then(|outer_box| outer_box.downcast::<Box>().ok())
                .and_then(|hbox| hbox.first_child()) // Get content box
                .and_then(|content_box| content_box.downcast::<Box>().ok())
                .and_then(|vbox| vbox.first_child()) // Get title label
                .and_then(|widget| widget.downcast::<Label>().ok())
                .map(|label| label.label().to_string());

            if let Some(title) = title {
                let notes_dir = crate::utils::get_notes_dir();
                let file_path = notes_dir.join(format!("{}.md", title));

                // When loading a note, update the window title properly
                match Note::load(&file_path) {
                    Ok(note) => {
                        PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = true);
                        text_view_for_select.buffer().set_text(&note.content);
                        PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = false);
                        *active_note_for_select.borrow_mut() = Some(ActiveNote {
                            path: file_path,
                            title: title.to_string(),
                            has_changes: false,
                            auto_save_source_id: None,
                            note: note.clone(),
                        });
                        window_for_select.set_title(Some(&format!("{} - {}", APP_NAME, title)));
                        let word_count = count_words(&note.content);
                        let count_text = format!("{} words", word_count);
                        status_label_for_select.set_text("Ready"); // Reset status
                        word_count_label_for_select.set_text(&count_text);
                        text_view_for_select.grab_focus(); // Focus editor
                    },
                    Err(e) => {
                        eprintln!("Error loading note content: {}", e);
                        PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = true);
                        text_view_for_select.buffer().set_text("");
                        PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = false);
                        window_for_select.set_title(Some(&format!("{}", APP_NAME))); // Just use app name
                        status_label_for_select.set_text("Error loading note");
                        *active_note_for_select.borrow_mut() = None;
                        text_view_for_select.grab_focus(); // Focus editor even on error
                    }
                }
            } else {
                 // Handle case where title couldn't be extracted (e.g., placeholder row)
                *active_note_for_select.borrow_mut() = None;
                PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = true);
                text_view_for_select.buffer().set_text("");
                PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = false);
                window_for_select.set_title(Some("JustWrite"));
                status_label_for_select.set_text("Ready");
                word_count_label_for_select.set_text("0 words");
                text_view_for_select.grab_focus(); // Focus editor
            }
        } else {
            // No row selected
            PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = true);
            text_view_for_select.buffer().set_text("");
            PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = false);
            *active_note_for_select.borrow_mut() = None;
            window_for_select.set_title(Some("JustWrite"));
            status_label_for_select.set_text("Ready");
            word_count_label_for_select.set_text("0 words");
            text_view_for_select.grab_focus(); // Focus editor
        }
        // No need to call update_ui_for_selection here, as header buttons are removed
    });

    // --- Text Buffer Change Logic with fixed borrowing ---
    let buffer = text_view.buffer();
    let active_note_for_changes = active_note.clone();
    let text_view_for_changes = text_view.clone();
    let status_label_for_changes = status_label.clone();
    let word_count_label_for_changes = word_count_label.clone();
    // Make these available for refresh_note_list in auto-save
    let list_box_for_auto_save_refresh = list_box.clone(); 
    let window_for_auto_save_refresh = window.clone();
    let text_view_for_auto_save_refresh = text_view.clone(); // text_view_ref for refresh_note_list

    buffer.connect_changed(move |_| {
        if PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow()) {
            return; // Ignore changes made by set_text programmatically
        }

        let content = text_view_for_changes.buffer().text(
            &text_view_for_changes.buffer().start_iter(),
            &text_view_for_changes.buffer().end_iter(),
            false
        ).to_string();
        
        // Update word count immediately regardless of active note state
        let word_count = count_words(&content);
        let count_text = format!("{} words", word_count);
        word_count_label_for_changes.set_text(&count_text);
        
        // Separate mutable borrow scope to avoid conflicts
        let mut _update_title = false; // Prefixed
        let mut _new_title = String::new(); // Prefixed
        let mut _need_refresh = false; // Prefixed and remove mut if not reassigned later
        
        {
            let mut active_note_guard = active_note_for_changes.borrow_mut();
            if let Some(active) = active_note_guard.as_mut() {
                active.has_changes = true;
                active.note.content = content.clone();
                status_label_for_changes.set_text("Editing...");
                
                // Cancel any existing auto-save
                if let Some(source_id) = active.auto_save_source_id.take() {
                    let _ = source_id.remove();
                }
                
                // Check for title update (outside active auto-save code)
                if active.note.title.starts_with("Note 20") { // Using direct check instead of has_default_title
                    // Simple title generation logic inline
                    if let Some(first_line) = content.lines().find(|line| !line.trim().is_empty()) {
                        let title_text = first_line.trim();
                        if title_text.len() >= 3 {
                            let new_potential_title = if title_text.len() > 50 {
                                format!("{}...", &title_text[0..47])
                            } else {
                                title_text.to_string()
                            };
                            
                            if new_potential_title != active.title && !new_potential_title.is_empty() {
                                _update_title = true;
                                _new_title = new_potential_title;
                            }
                        }
                    }
                }
                
                // Schedule a new auto-save
                let mut note_to_save = active.note.clone();
                let active_note_ref = active_note_for_changes.clone();
                let status_label_ref = status_label_for_changes.clone();
                // Clones for refresh_note_list inside auto-save
                let list_box_clone = list_box_for_auto_save_refresh.clone();
                let active_note_clone_for_refresh = active_note_for_changes.clone();
                let window_clone_for_refresh = window_for_auto_save_refresh.clone();
                let status_label_clone_for_refresh = status_label_for_changes.clone();
                let word_count_label_clone_for_refresh = word_count_label_for_changes.clone();
                let text_view_clone_for_refresh = text_view_for_auto_save_refresh.clone();
                
                active.auto_save_source_id = Some(schedule_auto_save(AUTO_SAVE_DELAY_MS, move || {
                    match note_to_save.save() {
                        Ok(_) => {
                            status_label_ref.set_text("Auto-saved");
                            let mut needs_list_refresh = false;
                            if let Ok(mut guard) = active_note_ref.try_borrow_mut() {
                                if let Some(active_inner) = guard.as_mut() {
                                    if active_inner.path == note_to_save.path { 
                                        active_inner.note.modified_time = note_to_save.modified_time;
                                        if active_inner.note.content == note_to_save.content {
                                            if active_inner.has_changes { // Only refresh if it *was* changed
                                                needs_list_refresh = true;
                                            }
                                            active_inner.has_changes = false;
                                        } 
                                        // else: content changed after this auto-save was scheduled, has_changes remains true
                                        active_inner.auto_save_source_id = None; 
                                    }
                                }
                            }
                            
                            if needs_list_refresh {
                                refresh_note_list(&list_box_clone, &active_note_clone_for_refresh, &window_clone_for_refresh, &status_label_clone_for_refresh, &word_count_label_clone_for_refresh, &text_view_clone_for_refresh);
                                // Potentially re-select the active note to ensure its row is visible/updated if order changed
                                if let Ok(guard) = active_note_clone_for_refresh.try_borrow() {
                                    if let Some(active_inner) = guard.as_ref() {
                                        select_note_by_title(&list_box_clone, &active_inner.title);
                                    }
                                }
                            }

                            let status_label_clone = status_label_ref.clone();
                            glib::timeout_add_seconds_local(3, move || {
                                status_label_clone.set_text("Ready");
                                glib::ControlFlow::Break
                            });
                        },
                        Err(e) => {
                            eprintln!("Auto-save error: {}", e);
                            status_label_ref.set_text("Auto-save failed");
                        }
                    }
                }));
            }
        }
        
        // Handle title update outside the borrow scope if needed
        /*
        if _update_title {
            // Get a new borrow to update the title
            if let Ok(mut active_guard) = active_note_for_changes.try_borrow_mut() {
                if let Some(active) = active_guard.as_mut() {
                    if let Ok(()) = active.note.rename(&_new_title) {
                        active.title = _new_title.clone();
                        window_for_auto_save_refresh.set_title(Some(&format!("{} - {}", APP_NAME, _new_title))); // Use cloned window
                        _need_refresh = true;
                    }
                }
            }
            
            // Refresh list if title was updated
            if _need_refresh {
                // Use a timeout to delay the refresh slightly
                let list_box_clone = list_box_for_auto_save_refresh.clone();
                let title_clone = _new_title.clone();
                let active_note_clone_for_title_refresh = active_note_for_changes.clone();
                let window_clone_for_title_refresh = window_for_auto_save_refresh.clone();
                let status_label_clone_for_title_refresh = status_label_for_changes.clone();
                let word_count_label_clone_for_title_refresh = word_count_label_for_changes.clone();
                let text_view_clone_for_title_refresh = text_view_for_auto_save_refresh.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    refresh_note_list(&list_box_clone, &active_note_clone_for_title_refresh, &window_clone_for_title_refresh, &status_label_clone_for_title_refresh, &word_count_label_clone_for_title_refresh, &text_view_clone_for_title_refresh);
                    select_note_by_title(&list_box_clone, &title_clone);
                    glib::ControlFlow::Break
                });
            }
        }
        */
    });

    // --- Global Keyboard Shortcuts ---
    let key_controller = EventControllerKey::new();
    let fullscreen_button_for_key = fullscreen_button.clone();
    let theme_toggle_for_key = theme_toggle_button.clone();
    let sidebar_toggle_for_key = sidebar_toggle.clone();
    let window_for_key = window.clone();
    let shortcuts_button_for_key = shortcuts_button.clone();
    
    key_controller.connect_key_pressed(move |_, key, _keycode, state| {
        // F11 for fullscreen
        if key == Key::F11 {
            fullscreen_button_for_key.emit_clicked();
            return Propagation::Stop;
        }
        
        // Ctrl+T for theme toggle
        if key == Key::t && state.contains(ModifierType::CONTROL_MASK) {
            theme_toggle_for_key.emit_clicked();
            return Propagation::Stop;
        }
        
        // Ctrl+B for sidebar toggle
        if key == Key::b && state.contains(ModifierType::CONTROL_MASK) {
            sidebar_toggle_for_key.emit_clicked();
            return Propagation::Stop;
        }
        
        // Ctrl+K for keyboard shortcuts dialog
        if key == Key::k && state.contains(ModifierType::CONTROL_MASK) {
            shortcuts_button_for_key.emit_clicked();
            return Propagation::Stop;
        }
        
        // Escape to exit fullscreen
        if key == Key::Escape && window_for_key.is_fullscreen() {
            window_for_key.unfullscreen();
            fullscreen_button_for_key.set_icon_name("view-fullscreen-symbolic");
            return Propagation::Stop;
        }
        
        Propagation::Proceed
    });
    
    window.add_controller(key_controller.clone());

    // --- New Note Button Logic ---
    let list_box_for_new = list_box.clone();
    let active_note_for_new = active_note.clone();
    let text_view_for_new = text_view.clone();
    let window_for_new = window.clone();
    let status_label_for_new = status_label.clone();
    let word_count_label_for_new = word_count_label.clone(); // Clone for new note
    let text_view_for_new_refresh = text_view.clone(); // Clone for refresh_note_list call

    new_note_button.connect_clicked(move |_| {
        // Synchronously save the currently active note if it has changes
        {
            let mut active_opt = active_note_for_new.borrow_mut(); // This is a clone of the main active_note Rc<RefCell>
            if let Some(active) = active_opt.as_mut() {
                if active.has_changes {
                    if let Some(source_id) = active.auto_save_source_id.take() {
                        let _ = source_id.remove();
                    }
                    match active.note.save() {
                        Ok(_) => {
                            active.has_changes = false;
                            status_label_for_new.set_text("Saved"); // Give feedback

                            // Refresh the list to update preview/timestamp for the saved note
                            let list_box_for_refresh = list_box_for_new.clone();
                            let active_note_for_refresh = active_note_for_new.clone(); // This Rc still points to the *previously* active note
                            let window_for_refresh = window_for_new.clone();
                            let status_label_for_refresh = status_label_for_new.clone();
                            let word_count_label_for_refresh = word_count_label_for_new.clone();
                            let text_view_for_refresh = text_view_for_new_refresh.clone(); // Use the one cloned for new note logic
                            let _original_title_for_reselect = active.title.clone(); // THIS LINE: Ensure variable is _original_title_for_reselect

                            refresh_note_list(&list_box_for_refresh, &active_note_for_refresh, &window_for_refresh, &status_label_for_refresh, &word_count_label_for_refresh, &text_view_for_refresh);
                            // No select_note_by_title using this variable here

                            let status_label_clone = status_label_for_new.clone();
                             glib::timeout_add_seconds_local(2, move || {
                                if status_label_clone.text() == "Saved" {
                                    status_label_clone.set_text("Ready");
                                }
                                glib::ControlFlow::Break
                            });
                        }
                        Err(e) => {
                            eprintln!("Error saving note {} before creating new: {}", active.title, e);
                            show_error_dialog(&window_for_new, "Save Error", &format!("Failed to save changes to note '{}' before creating new: {}. Your changes were NOT saved. New note not created.", active.title, e));
                             // Note: active.has_changes remains true, as the save failed.
                            return; // CRITICAL: Do not proceed with new note creation if save failed
                        }
                    }
                } else {
                    if let Some(source_id) = active.auto_save_source_id.take() {
                        let _ = source_id.remove();
                    }
                }
            }
        }

        // Find an empty note or create a new one
        match find_or_create_new_note() {
            Ok(note) => { // note is no longer mut here as update_title_if_empty_and_old is removed
                // Clear the editor
                PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = true);
                text_view_for_new.buffer().set_text(&note.content); // Use content from the new note
                PROGRAMMATIC_TEXT_CHANGE.with(|ptc| *ptc.borrow_mut() = false);
                
                // Update the active note
                *active_note_for_new.borrow_mut() = Some(ActiveNote {
                    path: note.path.clone(),
                    title: note.title.clone(),
                    has_changes: false, // Start fresh
                    auto_save_source_id: None,
                    note: note.clone(),
                });
                
                window_for_new.set_title(Some(&format!("{} - {}", APP_NAME, note.title)));
                let word_count = count_words(&note.content);
                status_label_for_new.set_text("Ready");
                word_count_label_for_new.set_text(&format!("{} words", word_count));
                
                refresh_note_list(&list_box_for_new, &active_note_for_new, &window_for_new, &status_label_for_new, &word_count_label_for_new, &text_view_for_new_refresh);
                select_note_by_title(&list_box_for_new, &note.title);
                text_view_for_new.grab_focus(); // Focus editor after creating and selecting new note
            },
            Err(e) => {
                eprintln!("Error finding or creating new note: {}", e);
                show_error_dialog(&window_for_new, "Create Error", &format!("Failed to create new note: {}", e));
            }
        }
    });

    // Remove rename_button_ref and delete_button_ref connections as they are moved to rows

    // Add panes to the main container
    paned.set_start_child(Some(&left_pane));
    paned.set_end_child(Some(&right_pane));
    
    // Set the Overlay as the child of the window
    window.set_child(Some(&main_overlay));

    // Populate the notes list
    refresh_note_list(&list_box, &active_note, &window, &status_label, &word_count_label, &text_view);

    // Present the window to the user
    window.present();
}

/// Count words in text
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Find an empty note or create a new one, updating title if necessary
fn find_or_create_new_note() -> Result<Note, String> {
    // Always create a new note to prevent accidental reuse/rename of existing empty notes.
    Note::new(&Note::generate_unique_title())
}

/// Refresh the note list with edit and delete buttons on hover
fn refresh_note_list(list_box: &ListBox, active_note_ref: &Rc<RefCell<Option<ActiveNote>>>, window_ref: &ApplicationWindow, status_label_ref: &Label, word_count_label_ref: &Label, text_view_ref: &TextView) {
    // Remove all existing rows
    while let Some(row) = list_box.row_at_index(0) {
        list_box.remove(&row);
    }

    // Get all notes (already sorted by Note::get_all)
    match Note::get_all() {
        Ok(notes) => {
            let mut found_notes = false;

            // Add each note to the list
            for note in notes {
                found_notes = true;

                // Create a horizontal box for the row to hold content and controls
                let row_outer_box = Box::builder()
                    .orientation(Orientation::Horizontal)
                    .hexpand(true)
                    .css_classes(vec!["note-row-outer"]) // Add class for hover detection
                    .build();

                // Create labels for title, date, and preview in a vertical box
                let row_content_box = Box::builder()
                    .orientation(Orientation::Vertical)
                    .spacing(2)
                    .margin_start(12)
                    .margin_end(6) // Reduce right margin for a cleaner look
                    .margin_top(6)
                    .margin_bottom(6)
                    .hexpand(true)
                    .css_classes(vec!["note-content-box"])
                    .build();

                let title_label = Label::builder()
                    .label(&note.title)
                    .xalign(0.0)
                    .css_classes(vec!["note-title"])
                    .halign(gtk::Align::Start)
                    .build();

                // Format date as "Mon DD"
                let date_str = note.modified_time
                    .map(|st| {
                        let dt: DateTime<Local> = st.into();
                        dt.format("%b %d").to_string()
                    })
                    .unwrap_or_else(|| "-".to_string());

                let date_label = Label::builder()
                    .label(&date_str)
                    .xalign(0.0)
                    .css_classes(vec!["note-date", "dim-label"])
                    .halign(gtk::Align::Start)
                    .build();

                // Shorter preview text
                let preview_text = if note.is_empty() { // Use is_empty method
                    "Empty".to_string()
                } else {
                    note.content
                        .split_whitespace()
                        .take(5)
                        .collect::<Vec<&str>>()
                        .join(" ") + "..."
                };

                let preview_label = Label::builder()
                    .label(&preview_text)
                    .xalign(0.0)
                    .css_classes(vec!["note-preview", "dim-label"])
                    .halign(gtk::Align::Start)
                    .build();

                // Add labels to the content box
                row_content_box.append(&title_label);
                row_content_box.append(&date_label);
                row_content_box.append(&preview_label);

                // Style the control box
                let control_box = Box::builder()
                    .orientation(Orientation::Horizontal)
                    .valign(gtk::Align::Center)
                    .halign(gtk::Align::End)
                    .spacing(2) // Reduced spacing
                    .margin_end(6) // Smaller margin
                    .css_classes(vec!["note-controls"])
                    .build();

                // Edit button - minimal style, just the icon
                let edit_button = Button::builder()
                    .icon_name("document-edit-symbolic")
                    .tooltip_text("Rename Note")
                    .css_classes(vec!["icon-only-button"])
                    .build();

                // Delete button - with icon-only styling
                let delete_button = Button::builder()
                    .icon_name("user-trash-symbolic")
                    .tooltip_text("Delete Note")
                    .css_classes(vec!["icon-only-button"])
                    .build();

                // Add buttons to the control box
                control_box.append(&edit_button);
                control_box.append(&delete_button);

                // Make the controls container a bit sleeker
                control_box.set_opacity(0.7); // Slightly transparent by default

                // Add content and controls to the row
                row_outer_box.append(&row_content_box);
                row_outer_box.append(&control_box);

                // Create the row and add the content with subtler styling
                let row = gtk::ListBoxRow::builder()
                    .css_classes(vec!["note-row", "borderless"])
                    .build();
                
                row.set_child(Some(&row_outer_box));

                // Store note title in row data for button handlers
                let note_title = note.title.clone();
                
                // Clone necessary Rcs and other variables for the closures
                let active_note_clone_for_edit = active_note_ref.clone();
                let window_clone_for_edit = window_ref.clone(); // ApplicationWindow is Clone
                let status_label_clone_for_edit = status_label_ref.clone();
                let note_title_for_edit_outer = note_title.clone(); // Used to identify if active note is this one

                // Clones for the on_confirm callback of show_rename_dialog
                let active_note_clone_for_rename_confirm = active_note_ref.clone();
                let window_clone_for_rename_confirm = window_ref.clone();
                let status_label_clone_for_rename_confirm = status_label_ref.clone();
                let list_box_clone_for_rename_confirm = list_box.clone();
                // We'll also need word_count_label and text_view for later potential updates if active note changes
                let word_count_label_clone_for_rename_confirm = word_count_label_ref.clone();
                let text_view_clone_for_rename_confirm = text_view_ref.clone();

                let row_for_button_closure = row.clone(); // Capture the current row for the button

                edit_button.connect_clicked(move |_| {
                    let mut proceed_with_rename = true;
                    // Check if the note to be renamed is the active note and has changes
                    if let Some(active_note_guard) = active_note_clone_for_edit.borrow().as_ref() {
                        if active_note_guard.title == note_title_for_edit_outer && active_note_guard.has_changes {
                            // Attempt to save
                            if let Some(active_mut_guard) = active_note_clone_for_edit.borrow_mut().as_mut() {
                                match active_mut_guard.note.save() {
                                    Ok(_) => {
                                        active_mut_guard.has_changes = false;
                                        status_label_clone_for_edit.set_text("Saved");

                                        // Refresh list after saving before rename
                                        let list_box_for_refresh = list_box_clone_for_rename_confirm.clone(); 
                                        let active_note_for_refresh = active_note_clone_for_edit.clone(); 
                                        let window_for_refresh = window_clone_for_edit.clone();
                                        let status_label_for_refresh = status_label_clone_for_edit.clone();
                                        let word_count_label_for_refresh = word_count_label_clone_for_rename_confirm.clone(); 
                                        let text_view_for_refresh = text_view_clone_for_rename_confirm.clone(); 
                                        let _original_title_for_reselect = active_mut_guard.title.clone(); // Ensure not prefixed

                                        refresh_note_list(&list_box_for_refresh, &active_note_for_refresh, &window_for_refresh, &status_label_for_refresh, &word_count_label_for_refresh, &text_view_for_refresh);
                                        select_note_by_title(&list_box_for_refresh, &_original_title_for_reselect); // Ensure uses non-prefixed

                                        let status_label_reset = status_label_clone_for_edit.clone();
                                        glib::timeout_add_seconds_local(2, move || {
                                            if status_label_reset.text() == "Saved" {
                                                status_label_reset.set_text("Ready");
                                            }
                                            glib::ControlFlow::Break
                                        });
                                    }
                                    Err(e) => {
                                        show_error_dialog(
                                            &window_clone_for_edit,
                                            "Save Error",
                                            &format!("Failed to save changes to note '{}' before renaming: {}. Rename aborted.", active_mut_guard.title, e),
                                        );
                                        proceed_with_rename = false;
                                    }
                                }
                            }
                        }
                    }

                    if !proceed_with_rename {
                        return; // Abort rename
                    }

                    if let Some(window) = row_for_button_closure.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                        if let Ok(app_window) = window.downcast::<ApplicationWindow>() {
                            let note_title_for_dialog = note_title_for_edit_outer.clone(); // Title to show in dialog
                            let original_title_for_confirm = note_title_for_edit_outer.clone(); // Original title to identify note

                            show_rename_dialog(
                                &app_window,
                                note_title_for_dialog,
                                clone!(@strong list_box_clone_for_rename_confirm, 
                                       @strong original_title_for_confirm,
                                       @strong active_note_clone_for_rename_confirm,
                                       @strong window_clone_for_rename_confirm,
                                       @strong status_label_clone_for_rename_confirm,
                                       @strong word_count_label_clone_for_rename_confirm,
                                       @strong text_view_clone_for_rename_confirm => move |new_title| {
                                    let notes_dir = crate::utils::get_notes_dir();
                                    let original_file_path = notes_dir.join(format!("{}.md", original_title_for_confirm));

                                    match Note::load(&original_file_path) {
                                        Ok(mut note_to_rename) => {
                                            if let Err(e) = note_to_rename.rename(&new_title) {
                                                show_error_dialog(&window_clone_for_rename_confirm, "Rename Failed", &format!("Could not rename the note: {}", e));
                                                return;
                                            }

                                            // If rename was successful, check if it was the active note
                                            let mut active_note_guard = active_note_clone_for_rename_confirm.borrow_mut();
                                            if let Some(active) = active_note_guard.as_mut() {
                                                if active.title == original_title_for_confirm {
                                                    active.title = new_title.clone();
                                                    active.path = note_to_rename.path.clone();
                                                    active.note.title = new_title.clone();
                                                    active.note.path = note_to_rename.path.clone();
                                                    active.note.modified_time = note_to_rename.modified_time;
                                                    active.has_changes = false; // Changes were saved before rename or it's a fresh state
                                                    window_clone_for_rename_confirm.set_title(Some(&format!("{} - {}", APP_NAME, new_title)));
                                                    status_label_clone_for_rename_confirm.set_text("Renamed");
                                                     let status_label_reset = status_label_clone_for_rename_confirm.clone();
                                                    glib::timeout_add_seconds_local(2, move || {
                                                        if status_label_reset.text() == "Renamed" {
                                                           status_label_reset.set_text("Ready");
                                                        }
                                                        glib::ControlFlow::Break
                                                    });
                                                }
                                            }
                                            drop(active_note_guard); // Release borrow

                                            refresh_note_list(&list_box_clone_for_rename_confirm, &active_note_clone_for_rename_confirm, &window_clone_for_rename_confirm, &status_label_clone_for_rename_confirm, &word_count_label_clone_for_rename_confirm, &text_view_clone_for_rename_confirm);
                                            select_note_by_title(&list_box_clone_for_rename_confirm, &new_title);
                                        }
                                        Err(e) => {
                                             show_error_dialog(&window_clone_for_rename_confirm, "Rename Failed", &format!("Could not load note for renaming: {}", e));
                                        }
                                    }
                                })
                            );
                        }
                    }
                });
                
                // Connect the delete button to show confirmation dialog
                let list_box_for_delete = list_box.clone();
                let row_for_delete = row.clone();
                let note_title_for_delete = note_title.clone(); // Clone for delete button
                let active_note_clone_for_delete = active_note_ref.clone();
                let window_clone_for_delete = window_ref.clone();
                let status_label_clone_for_delete = status_label_ref.clone();
                let word_count_label_clone_for_delete = word_count_label_ref.clone();
                let text_view_clone_for_delete = text_view_ref.clone();
                
                delete_button.connect_clicked(move |_| {
                    if let Some(window) = row_for_delete.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                        // Ensure window is ApplicationWindow
                        if let Ok(app_window) = window.downcast::<ApplicationWindow>() {
                            show_confirmation_dialog(
                                &app_window, // Pass correct type
                                "Confirm Deletion",
                                &format!("Delete note \"{}\"?", note_title_for_delete),
                                "This action cannot be undone.",
                                clone!(@strong list_box_for_delete, 
                                       @strong note_title_for_delete, 
                                       @strong active_note_clone_for_delete, 
                                       @strong window_clone_for_delete, 
                                       @strong status_label_clone_for_delete, 
                                       @strong word_count_label_clone_for_delete, 
                                       @strong text_view_clone_for_delete => move || {
                                    // Get the current note using cloned title
                                    if let Ok(note) = Note::load(&crate::utils::get_notes_dir()
                                        .join(format!("{}.md", note_title_for_delete))) {
                                        // Delete it
                                        if let Ok(()) = note.delete() {
                                            // Refresh the list
                                            refresh_note_list(&list_box_for_delete, &active_note_clone_for_delete, &window_clone_for_delete, &status_label_clone_for_delete, &word_count_label_clone_for_delete, &text_view_clone_for_delete);
                                            // Optionally clear editor if deleted note was active
                                            // This will be handled by the data consistency fix for delete: Issue #2
                                            // If the deleted note was active, clear the editor and active_note state.
                                            let active_note_guard = active_note_clone_for_delete.borrow_mut(); // Removed mut from let binding
                                            if let Some(active) = active_note_guard.as_ref() {
                                                if active.title == note_title_for_delete {
                                                    // Drop the mutable borrow before calling text_view_clone_for_delete methods or other functions that might borrow active_note
                                                    drop(active_note_guard);

                                                    text_view_clone_for_delete.buffer().set_text("");
                                                    *active_note_clone_for_delete.borrow_mut() = None;
                                                    window_clone_for_delete.set_title(Some(APP_NAME));
                                                    status_label_clone_for_delete.set_text("Ready");
                                                    word_count_label_clone_for_delete.set_text("0 words");
                                                    // No specific note to select, list_box.connect_row_selected will handle if selection clears or moves.
                                                    // text_view_clone_for_delete.grab_focus(); // Focus editor
                                                }
                                            }

                                        } else {
                                            // Handle delete error
                                            if let Some(root_window) = list_box_for_delete.root().and_then(|r| r.downcast::<ApplicationWindow>().ok()) {
                                                show_error_dialog(&root_window, "Delete Failed", "Could not delete the note.");
                                            }
                                        }
                                    }
                                })
                            );
                        }
                    }
                });

                // Add the row to the ListBox
                list_box.append(&row);
            }

            // If no notes were found, show a placeholder message
            if !found_notes {
                let label = Label::builder()
                    .label("No notes yet. Create one!")
                    .xalign(0.0)
                    .margin_start(12)
                    .margin_end(12)
                    .margin_top(12)
                    .margin_bottom(12)
                    .css_classes(vec!["no-notes-label"])
                    .build();
                
                let row = gtk::ListBoxRow::builder()
                    .css_classes(vec!["empty-note-row"])
                    .build();
                    
                row.set_child(Some(&label));
                row.set_sensitive(false); // Make it non-selectable
                list_box.append(&row);
            }
        },
        Err(e) => {
            eprintln!("Error reading notes: {}", e);
            // Add an error message to the list
            let label = Label::builder()
                .label("Error loading notes")
                .xalign(0.0)
                .margin_start(12)
                .margin_end(12)
                .margin_top(12)
                .margin_bottom(12)
                .css_classes(vec!["error-label"])
                .build();
            
            let row = gtk::ListBoxRow::builder()
                .css_classes(vec!["error-row"])
                .build();
                
            row.set_child(Some(&label));
            list_box.append(&row);
        }
    }
}

/// Load CSS styling for the application
fn load_css() {
    // Define possible CSS file locations
    let css_paths = [
        "/usr/share/penscript/style.css", // Updated for potential system-wide install
        "src/style.css",
        "style.css",
    ];
    
    // Find the first available CSS file
    if let Some(css_file) = css_paths.iter().find(|&path| std::path::Path::new(path).exists()) {
        match std::fs::read_to_string(css_file) {
            Ok(css_data) => {
                // Create CSS provider and load the CSS data
                let css_provider = CssProvider::new();
                css_provider.load_from_data(&css_data);
                
                // Apply to the default display using the new API
                if let Some(display) = gtk::gdk::Display::default() {
                    gtk::style_context_add_provider_for_display(
                        &display,
                        &css_provider,
                        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                    println!("Loaded CSS from {}", css_file);
                } else {
                    eprintln!("Warning: Could not get default display. CSS styling not applied.");
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to read CSS file {}: {}", css_file, e);
            }
        }
    } else {
        eprintln!("Warning: No CSS file found. Using default styling.");
    }
}

/// Shows a dialog to rename a note (modern GTK4, no deprecated APIs)
fn show_rename_dialog<F>(parent: &ApplicationWindow, current_title: String, on_confirm: F)
where
    F: Fn(String) + 'static,
{
    use gtk::{Orientation, Box as GtkBox, Entry, Button, Label, Align};

    // Create a new transient window as a modal dialog
    let dialog = ApplicationWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title("Rename Note")
        .default_width(320)
        .default_height(120)
        .css_classes(vec!["rename-dialog"])  // Add CSS class for styling
        .build();

    // Main vertical box
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(16);
    vbox.set_margin_bottom(16);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);

    let label = Label::new(Some("Enter new name:"));
    label.set_halign(Align::Start);
    vbox.append(&label);

    let entry = Entry::builder()
        .text(&current_title)
        .hexpand(true)
        .build();
    vbox.append(&entry);

    // Button row
    let button_box = GtkBox::new(Orientation::Horizontal, 8);
    button_box.set_halign(Align::End);

    let cancel_button = Button::with_label("Cancel");
    let rename_button = Button::with_label("Rename");
    button_box.append(&cancel_button);
    button_box.append(&rename_button);

    vbox.append(&button_box);
    dialog.set_child(Some(&vbox));

    // Focus entry and select text when shown
    let entry_clone = entry.clone();
    dialog.connect_map(move |_| {
        entry_clone.grab_focus();
        entry_clone.select_region(0, -1);
    });

    // Cancel closes the dialog
    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.close();
    });

    // Confirm logic
    let dialog_clone = dialog.clone();
    let parent_clone = parent.clone();
    let current_title_clone = current_title.clone();
    let entry_for_rename = entry.clone();
    rename_button.connect_clicked(move |_| {
        let new_title = entry_for_rename.text().to_string();
        if !new_title.trim().is_empty() && new_title != current_title_clone {
            dialog_clone.close();
            on_confirm(new_title);
        } else if new_title.trim().is_empty() {
            show_error_dialog(&parent_clone, "Rename Error", "New title cannot be empty.");
        } else {
            // User entered the same title - just close the dialog without an error
            dialog_clone.close();
        }
    });

    // Allow pressing Enter to confirm
    let rename_button_clone = rename_button.clone();
    let entry_for_activate = entry.clone();
    entry_for_activate.connect_activate(move |_| {
        rename_button_clone.emit_clicked();
    });

    dialog.present();
}

/// Find and select a note by its title
fn select_note_by_title(list_box: &ListBox, title_to_find: &str) {
    let mut row_index = 0;

    while let Some(row) = list_box.row_at_index(row_index) {
        // Find the title label within the row's Box (now vertical)
        let title_label_opt = row.child()
            .and_then(|child_box| child_box.downcast::<Box>().ok())
            .and_then(|vbox| vbox.first_child()) // Get the first child (title label)
            .and_then(|widget| widget.downcast::<Label>().ok());

        if let Some(title_label) = title_label_opt {
            if title_label.label() == title_to_find {
                list_box.select_row(Some(&row));
                // We don't call row.grab_focus() here anymore.
                // The text_view focus will be handled by the list_box.connect_row_selected callback.
                return;
            }
        }

        row_index += 1;
    }
}

/// Show keyboard shortcuts dialog
fn show_shortcuts_dialog(parent: &ApplicationWindow) {
    // Create a dialog window
    let dialog = ApplicationWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title("Keyboard Shortcuts")
        .default_width(400)
        .default_height(400)
        .css_classes(vec!["shortcuts-dialog"])
        .build();
    
    // Create a scrollable container
    let scrolled_window = ScrolledWindow::new();
    
    // Create the main box for content
    let content_box = Box::builder()
        .orientation(Orientation::Vertical)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .spacing(10)
        .build();
    
    // Add title
    let title_label = Label::builder()
        .label("Keyboard Shortcuts")
        .css_classes(vec!["shortcuts-title"])
        .build();
    content_box.append(&title_label);
    
    // Create sections
    add_shortcut_section(&content_box, "General", &[
        ("Ctrl+K", "Show keyboard shortcuts"),
        ("Ctrl+T", "Toggle light/dark theme"),
        ("Ctrl+B", "Toggle sidebar"),
        ("F11", "Toggle fullscreen mode"),
        ("Escape", "Exit fullscreen")
    ]);
    
    add_shortcut_section(&content_box, "Editing", &[
        ("Ctrl+S", "Save current note"),
        ("Ctrl+N", "Create new note"),
        ("Ctrl+D", "Delete selected note"),
        ("Ctrl+R", "Rename selected note")
    ]);
    
    // Add a close button
    let button_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(gtk::Align::End)
        .margin_top(20)
        .build();
        
    let close_button = Button::builder()
        .label("Close")
        .build();
        
    button_box.append(&close_button);
    content_box.append(&button_box);
    
    // Connect close button
    let dialog_clone = dialog.clone();
    close_button.connect_clicked(move |_| {
        dialog_clone.close();
    });
    
    // Set content
    scrolled_window.set_child(Some(&content_box));
    dialog.set_child(Some(&scrolled_window));
    
    // Show dialog
    dialog.present();
}

/// Helper to add a section of shortcuts to the dialog
fn add_shortcut_section(container: &Box, title: &str, shortcuts: &[(&str, &str)]) {
    // Add section title
    let section_label = Label::builder()
        .label(title)
        .xalign(0.0)
        .css_classes(vec!["shortcuts-section"])
        .margin_top(10)
        .build();
    container.append(&section_label);
    
    // Add shortcuts grid
    let grid = gtk::Grid::builder()
        .row_spacing(8)
        .column_spacing(20)
        .margin_start(20)
        .build();
    
    for (i, (key, description)) in shortcuts.iter().enumerate() {
        let key_label = Label::builder()
            .label(*key) // Dereference to solve the type issue
            .xalign(0.0)
            .css_classes(vec!["shortcut-key"])
            .build();
            
        let desc_label = Label::builder()
            .label(*description) // Dereference to solve the type issue
            .xalign(0.0)
            .build();
            
        grid.attach(&key_label, 0, i as i32, 1, 1);
        grid.attach(&desc_label, 1, i as i32, 1, 1);
    }
    
    container.append(&grid);
}
