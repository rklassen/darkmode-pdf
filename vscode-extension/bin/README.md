# Bundled Renderer Binaries

Each platform package should contain exactly one renderer binary in the matching
target directory.

Expected layout:

```text
bin/
  darwin-arm64/darkmode-pdf
  win32-x64/darkmode-pdf.exe
  win32-arm64/darkmode-pdf.exe
  linux-x64/darkmode-pdf
  linux-arm64/darkmode-pdf
```

The extension resolves the binary using `process.platform` and `process.arch`
and then applies the additional Linux distro gate for `ubuntu` and `arch`.
