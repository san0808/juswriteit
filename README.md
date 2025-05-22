# Penscript

A minimalist, native Linux desktop note-taking application designed for focused writing. Penscript stores your notes locally as Markdown files, providing a clean, distraction-free environment for your thoughts.

Built with Rust and GTK4, Penscript combines the performance and safety of Rust with the native look and feel of the GTK toolkit.

## Features

- **Minimalist Interface**: Clean, modern UI with light and dark themes
- **Distraction-Free Writing**: Focus on your content, not the interface
- **Local Markdown Storage**: All notes are stored as plain .md files you can access anytime
- **Rich Note Management**:
  - Create, edit, rename, and delete notes
  - Sort notes by modification date
  - Search and filter notes
  - Auto-save functionality
- **Modern Design Elements**:
  - Custom window frame with integrated controls
  - Responsive layout with resizable sidebar
  - Fullscreen mode for complete focus
  - Keyboard shortcuts for power users

## Installation

### From Source

1. Install Rust and Cargo:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Install GTK4 development libraries (system dependent):
   ```bash
   # For Ubuntu/Debian
   sudo apt install libgtk-4-dev
   
   # For Fedora
   sudo dnf install gtk4-devel
   
   # For Arch/Manjaro
   sudo pacman -S gtk4
   ```

3. Clone and build the project:
   ```bash
   git clone https://github.com/san0808/penscript.git
   cd penscript
   cargo build --release
   ```

4. Run the application:
   ```bash
   ./target/release/penscript
   ```

## Usage

### Basic Operations

- **Creating Notes**: Click the plus button in the sidebar header
- **Editing Notes**: Select a note from the sidebar and start typing in the editor
- **Saving Notes**: Notes are automatically saved when you pause typing
- **Searching Notes**: Type in the search bar above the note list to filter by title
- **Deleting Notes**: Hover over a note in the sidebar and click the trash icon
- **Renaming Notes**: Hover over a note in the sidebar and click the pencil icon

### Keyboard Shortcuts

- `Ctrl+B`: Toggle sidebar
- `Ctrl+T`: Toggle between light and dark themes
- `F11`: Toggle fullscreen mode
- `Escape`: Exit fullscreen mode
- `Ctrl+K`: Show keyboard shortcuts dialog

## File Storage

Notes are stored as Markdown (.md) files in:
```
~/.local/share/penscript/notes/
```

Each note is saved as a separate file, with the filename corresponding to the note title.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
