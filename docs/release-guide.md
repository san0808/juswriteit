# Release Guide for Penscript

This document outlines the process for creating and distributing releases of Penscript.

## Manual AppImage Creation

You can create an AppImage manually using the `build-appimage.sh` script:

```bash
# Make the script executable if it's not already
chmod +x build-appimage.sh

# Run the build script
./build-appimage.sh
```

The script will:
1. Build the application in release mode
2. Create an AppDir structure
3. Copy the binary and assets to the AppDir
4. Create necessary metadata
5. Generate the AppImage

When completed, you'll have a file named `Penscript-0.1.0-x86_64.AppImage` that users can run directly.

## GitHub Actions Automated Builds

The project is set up with GitHub Actions to automatically build AppImages:

1. For every push to `main`, an AppImage will be built and stored as an artifact
2. When tagging a release (format: `v*`), the AppImage will be automatically attached to the GitHub Release

### Creating a New Release

1. Update the version in `Cargo.toml`
2. Commit changes and push to GitHub
3. Create a new tag and release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

4. Go to the GitHub repository and create a new release from this tag
5. The GitHub Actions workflow will automatically attach the AppImage to the release

## Testing the AppImage

Before releasing, test the AppImage on various Linux distributions:

```bash
# Make the AppImage executable
chmod +x Penscript-0.1.0-x86_64.AppImage

# Run it
./Penscript-0.1.0-x86_64.AppImage
```

Verify that:
- The application starts correctly
- Notes can be created, edited, and deleted
- The UI looks correct
- Theme switching works
- Keyboard shortcuts function as expected 