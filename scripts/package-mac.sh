#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
"${CARGO:-$HOME/.cargo/bin/cargo}" build --release --features play --bin rocketpult
mkdir -p 'Rocketpult.app/Contents/MacOS'
cp target/release/rocketpult 'Rocketpult.app/Contents/MacOS/rocketpult'
cat > 'Rocketpult.app/Contents/Info.plist' <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>rocketpult</string>
<key>CFBundleIdentifier</key><string>local.rocketpult.rust</string>
<key>CFBundleName</key><string>Rocketpult Rust</string>
<key>CFBundleDisplayName</key><string>Rocketpult Classic · Rust</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
printf 'APPL????' > 'Rocketpult.app/Contents/PkgInfo'
printf '\nReady: Rocketpult.app\n'
