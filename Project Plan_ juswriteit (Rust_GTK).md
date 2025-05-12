# **Project Plan: Penscript**

Version: 1.0 (Initial Plan)
Date: 2025-04-25
Goal: A minimalist, native Linux desktop app for focused writing, storing notes locally, built with Rust and GTK, inspired by Freewrite's UI.

## **1\. Core Decisions**

* **Language:** Rust (for performance, memory safety, native binaries)
* **GUI Toolkit:** GTK4 via gtk4-rs and glib-rs bindings (for native Linux look & feel, modern features)
* **Data Storage:** Plain Markdown files (.md) stored in a dedicated directory (\~/.local/share/juswriteit/notes/ by default). Filenames act as titles.
* **Architecture:** Model-View-Controller (MVC) inspired pattern.
  * **Model:** Rust modules/structs handling file system operations (listing, reading, writing, deleting, renaming notes).
  * **View:** GTK4 widgets defined and laid out in Rust code (ApplicationWindow, Paned, ListView/ListBox, TextView, HeaderBar, etc.).
  * **Controller:** GTK signal handlers connecting UI events (button clicks, list selections, text changes) to Model logic and updating the View. Asynchronous operations (like auto-save) managed via glib's main context integration.
* **Build System:** Cargo (Rust's default build system and package manager)
* **Styling:** GTK CSS for theming and achieving the minimalist aesthetic.
* **Packaging:** Initial target: cargo build \--release for a native binary. Later: AppImage / Flatpak for wider distribution.

## **2\. Development Phases**

### **Phase 1: Foundation & MVP (Minimum Viable Product)**

* **Goal:** A basic, functional app to create, view, edit, and save notes.
* **Tasks:**
  - [x] **Project Setup:** Initialize cargo project, add gtk4-rs and glib-rs dependencies.
  - [x] **Basic Application Structure:** Implement Gtk::Application and Gtk::ApplicationWindow.
  - [x] **Two-Pane Layout:** Use Gtk::Paned to create the sidebar/editor split.
  - [x] **Note Directory Handling:** Create/detect \~/.local/share/juswriteit/notes/ on startup. Handle potential errors.
  - [x] **Note List (Simple):** Implement a Gtk::ListBox in the left pane. Populate it by reading .md filenames from the notes directory.
  - [x] **Editor Area:** Implement a Gtk::TextView within a Gtk::ScrolledWindow in the right pane.
  - [x] **Load Note:** Connect ListBox::row-selected signal to read the corresponding .md file content into the TextView's TextBuffer.
  - [x] **Save Note (Manual):** Implement saving the TextBuffer content back to the selected file (e.g., via Ctrl+S or a save button). Handle overwriting.
  - [x] **Create New Note:** Add a Gtk::Button (e.g., in a Gtk::HeaderBar) to create a new Untitled Note \[timestamp\].md, add it to the ListBox, and select it.
  - [x] **Basic Error Handling:** Use Result for file I/O and report critical errors (e.g., cannot create notes directory) gracefully (e.g., Gtk::MessageDialog).

### **Phase 2: Core Enhancements**

* **Goal:** Improve usability, add essential features, and refine the core loop.
* **Tasks:**
  - [x] **Note List Polish:**
     * [x] Display modification date alongside the title in the ListBox rows. (Requires reading file metadata).
     * [x] Sort list (e.g., by modification date, descending).
     * [ ] Consider migrating from ListBox to ListView with a Gio::ListStore model for better scalability and features if needed.
  - [x] **Delete Note:** Implement note deletion (e.g., via a button or context menu) with a confirmation dialog (Gtk::MessageDialog). Update the list and potentially clear the editor.
  - [x] **Rename Note:** Implement renaming (e.g., via context menu or double-click). Rename the file and update the list item.
  - [x] **Auto-Save:** Implement optional auto-saving triggered by TextBuffer::changed signal after a short delay (using glib::timeout\_add\_local\_once).
  - [x] **Status Bar Info:** Add a simple status bar (e.g., using Gtk::Box at the bottom) to show word count.
  - [x] **Basic Styling:** Apply initial GTK CSS to customize colors, fonts, and spacing for a cleaner look.

### **Phase 3: Polish & Production Readiness**

* **Goal:** Refine the UI/UX, add quality-of-life features, and prepare for distribution.
* **Tasks:**
  - [x] **Advanced Styling:** Use GTK CSS extensively to match the minimalist Freewrite aesthetic. Implement Light/Dark theme support.
     * [x] Created modern dark and light themes with careful typography
     * [x] Implemented centered "typewriter" style editing area with padding
     * [x] Added theme toggle button with appropriate icons
     * [x] Improved sidebar with note previews and better spacing
     * [x] Implemented frameless window design with auto-hiding window controls
     * [x] Added overlay controls at bottom of window (fullscreen, theme, sidebar, shortcuts)
     * [x] Added app branding watermark at bottom center
     * [x] Added hover controls (rename, delete) to sidebar notes
     * [x] Moved "New Note" button to sidebar header
     * [x] Implemented logic to reuse/rename empty notes
     * [x] Added keyboard shortcuts for all major functions
     * [x] Added keyboard shortcuts reference dialog (Ctrl+K)
  - [ ] **Editor Improvements:**
     * [ ] Basic font selection/size options (maybe via Gtk::FontButton in a settings popover).
     * [ ] Consider basic Markdown syntax highlighting (could be complex, might use an external crate if available or keep it simple).
  - [ ] **Search/Filter Notes:** Add a Gtk::SearchEntry to filter the note list based on title/filename.
  - [ ] **Settings:** Implement a simple settings mechanism (e.g., Gtk::PopoverMenu from a HeaderBar button) for options like auto-save toggle, font settings. Store settings locally (e.g., in \~/.config/juswriteit/settings.toml using serde and toml).
  - [ ] **Robust Error Handling:** Add more specific error dialogs and recovery options.
  - [ ] **Code Cleanup & Documentation:** Ensure code is well-commented, formatted (cargo fmt), and follows Rust best practices. Write README.md.
  - [x] **Licensing:** Choose and add an open-source license file (e.g., MIT).

### **Phase 4: Packaging & Release**

* **Goal:** Make the application easily installable on target Linux systems.
* **Tasks:**
  - [ ] **Build Scripting:** Ensure cargo build \--release produces an optimized binary.
  - [ ] **App Icon:** Create/add an application icon (.desktop file integration).
  - [ ] **AppImage:** Create a build process for generating an AppImage.
  - [ ] **Flatpak (Optional):** Create a Flatpak manifest for packaging.
  - [ ] **Testing:** Test thoroughly on Manjaro (Arch-based), Ubuntu (Debian-based), and potentially Fedora.
  - [ ] **GitHub Release:** Create a repository, push code, tag a v1.0 release, and upload packaged artifacts.

## **3\. Key Crates**

* gtk4: Core GTK4 widget library bindings.
* glib: Bindings for GLib (core data structures, main loop, asynchronous operations).
* tokio or async-std: Optional, if complex async logic beyond simple glib timeouts is needed.
* serde, toml: For loading/saving configuration files.
* chrono: For handling timestamps in filenames or metadata display.
* log, env\_logger: For adding application logging.