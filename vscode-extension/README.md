# Darkmode PDF VS Code Extension

This folder contains a desktop VS Code extension wrapper for the Rust
`darkmode-pdf` renderer.

## What it does

- adds a `Darkmode PDF: Render Current Markdown File` command
- shells out to a bundled platform-specific `darkmode-pdf` binary
- writes the PDF chosen in the save dialog
- launches the binary with the extension folder as its working directory so the
  bundled `assets/` fonts are available

## Supported runtime targets

The scaffold is intentionally limited to:

- `darwin-arm64`
- `win32-x64`
- `win32-arm64`
- `linux-x64`
- `linux-arm64`

Linux support is runtime-gated to distro IDs `ubuntu` and `arch` read from
`/etc/os-release`.

## Binary layout

Place the compiled renderer binary into one of these folders before packaging:

- `bin/darwin-arm64/darkmode-pdf`
- `bin/win32-x64/darkmode-pdf.exe`
- `bin/win32-arm64/darkmode-pdf.exe`
- `bin/linux-x64/darkmode-pdf`
- `bin/linux-arm64/darkmode-pdf`

You can stage binaries automatically with the Node helper in `scripts/`.

Build and stage with Cargo:

```bash
cd vscode-extension
node ./scripts/stage-binary.js --target darwin-arm64 --build
node ./scripts/stage-binary.js --target win32-x64 --build
node ./scripts/stage-binary.js --target win32-arm64 --build
node ./scripts/stage-binary.js --target linux-x64 --build
node ./scripts/stage-binary.js --target linux-arm64 --build
```

Or stage a binary produced elsewhere:

```bash
cd vscode-extension
node ./scripts/stage-binary.js --target linux-x64 --binary /path/to/darkmode-pdf
```

Convenience npm scripts are also defined:

```bash
cd vscode-extension
npm run stage:darwin-arm64
npm run stage:win32-x64
npm run stage:win32-arm64
npm run stage:linux-x64
npm run stage:linux-arm64
```

The extension package also needs an `assets/` folder containing the font files
required by the renderer.

## Packaging model

Package this extension as a platform-specific desktop extension rather than a
web extension. Publish one VSIX per target platform so VS Code can select the
correct binary package for the host.

Example packaging commands:

```bash
vsce package --target darwin-arm64
vsce package --target win32-x64
vsce package --target win32-arm64
vsce package --target linux-x64
vsce package --target linux-arm64
```
