#!/bin/bash
set -e

# Configuration
APP_NAME="Penscript"
BINARY_NAME="juswriteit"
APP_ID="dev.penscript.Penscript"
ICON_NAME="com.github.san0808.penscript"
VERSION="0.1.0"
MAINTAINER="Sanket Bhat"
EMAIL="sanketbhat882002@gmail.com"

echo "🚀 Building AppImage for $APP_NAME ($VERSION)"

# Step 1: Build the application in release mode
echo "📦 Building release binary..."
cargo build --release

# Step 2: Create AppDir structure
echo "📁 Creating AppDir structure..."
rm -rf AppDir
mkdir -p AppDir/usr/bin
mkdir -p AppDir/usr/share/applications
mkdir -p AppDir/usr/share/icons/hicolor/scalable/apps
mkdir -p AppDir/usr/share/metainfo

# Step 3: Copy files to AppDir
echo "📋 Copying files to AppDir..."
cp target/release/$BINARY_NAME AppDir/usr/bin/
cp assets/dextop/penscript.desktop AppDir/usr/share/applications/$APP_ID.desktop
cp assets/icons/$ICON_NAME.svg AppDir/usr/share/icons/hicolor/scalable/apps/
cp assets/icons/$ICON_NAME.svg AppDir/.DirIcon
# Copy icon to AppDir root with the name expected by appimagetool
cp assets/icons/$ICON_NAME.svg AppDir/$ICON_NAME.svg

# Step 4: Create AppRun file
echo "📄 Creating AppRun file..."
cat > AppDir/AppRun << EOF
#!/bin/sh
SELF=\$(readlink -f "\$0")
HERE=\${SELF%/*}
export PATH="\${HERE}/usr/bin/:\${PATH}"
export LD_LIBRARY_PATH="\${HERE}/usr/lib/:\${LD_LIBRARY_PATH}"
exec "\${HERE}/usr/bin/$BINARY_NAME" "\$@"
EOF
chmod +x AppDir/AppRun

# Step 5: Create AppStream metadata
echo "📝 Creating AppStream metadata..."
cat > AppDir/usr/share/metainfo/$APP_ID.appdata.xml << EOF
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>$APP_ID</id>
  <metadata_license>MIT</metadata_license>
  <project_license>MIT</project_license>
  <name>$APP_NAME</name>
  <summary>A minimalist GTK4 writing application</summary>
  
  <description>
    <p>
      A minimalist, native Linux desktop app for focused writing, 
      storing notes locally, built with Rust and GTK, inspired by Freewrite's UI.
    </p>
  </description>
  
  <launchable type="desktop-id">$APP_ID.desktop</launchable>
  
  <url type="homepage">https://github.com/san0808/penscript</url>
  
  <provides>
    <binary>$BINARY_NAME</binary>
  </provides>
  
  <releases>
    <release version="$VERSION" date="$(date +%Y-%m-%d)">
      <description>
        <p>Initial release</p>
      </description>
    </release>
  </releases>
  
  <content_rating type="oars-1.1" />
  
  <developer id="github.com/san0808">
    <name>$MAINTAINER</name>
  </developer>
</component>
EOF

# Step 6: Fix desktop file paths
echo "🔧 Updating desktop file..."
sed -i 's|Exec=.*|Exec=juswriteit|g' AppDir/usr/share/applications/$APP_ID.desktop
sed -i "s|Icon=.*|Icon=$ICON_NAME|g" AppDir/usr/share/applications/$APP_ID.desktop

# Create symlink in AppDir root (needed for appimagetool)
echo "🔗 Creating desktop file symlink..."
ln -sf usr/share/applications/$APP_ID.desktop AppDir/$APP_ID.desktop

# Step 7: Download appimagetool if not present
if [ ! -f appimagetool ]; then
  echo "📥 Downloading appimagetool..."
  wget -c "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" -O appimagetool
  chmod +x appimagetool
fi

# Step 8: Create the AppImage
echo "🔨 Creating AppImage..."
chmod +x appimagetool
./appimagetool --appimage-extract-and-run AppDir "${APP_NAME}-${VERSION}-x86_64.AppImage"

echo "✅ AppImage created: ${APP_NAME}-${VERSION}-x86_64.AppImage"
echo "You can now test it by running: ./${APP_NAME}-${VERSION}-x86_64.AppImage" 