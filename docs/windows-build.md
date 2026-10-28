# Windows portable build and reboot test

## Produced on Ubuntu

- Portable folder: `dist/RetinaCare/`
- Transfer archive: `dist/RetinaCare-Windows-x86_64.zip`
- Executable: `dist/RetinaCare/RetinaCare.exe`
- Automated test: `dist/RetinaCare/test-windows.ps1`

The executable is a real x86-64 Windows GUI PE built with the GNU Windows target. It has an embedded icon and resource section. It has not been executed on Windows yet.

Before reboot, copy the ZIP and its `.sha256` file to an NTFS shared partition or USB drive. Windows does not normally expose Ubuntu ext4 paths. In Windows, verify the checksum, extract the whole `RetinaCare` folder, and keep its layout unchanged.

```powershell
Get-FileHash .\RetinaCare-Windows-x86_64.zip -Algorithm SHA256
Expand-Archive .\RetinaCare-Windows-x86_64.zip -DestinationPath .\RetinaCare-Test
Set-ExecutionPolicy -Scope Process Bypass
& .\RetinaCare-Test\RetinaCare\test-windows.ps1
```

The script verifies packaged files, Python-worker IPC, SQLCipher round-trip, all seven GUI screens, clean process exit, and no orphan packaged Python process. Reports are written beside the executable.

## Manual Windows checklist

1. Launch `RetinaCare.exe`; confirm icon, title, native rendering, and clean exit.
2. Open Dashboard, Patients, Screening workspace, Examination history, AI results, Settings, and GPU performance.
3. Unlock a new local database, add a synthetic patient and examination, close, reopen, and confirm persistence. Do not use real patient data for this pre-release test.
4. Import a non-sensitive retinal test image; verify display, zoom, pan, and oversized/invalid-file errors.
5. Confirm `%LOCALAPPDATA%\RetinaCare\database` and `logs\worker.log` exist.
6. Confirm health reports the RTX 5070 and CUDA only if the packaged Python runtime has a compatible PyTorch CUDA build.
7. Put approved model weights only in `%LOCALAPPDATA%\RetinaCare\models`; record checksum/license. Current worker reports `available_but_no_adapter` because trained inference integration is not implemented.
8. Save smoke JSON, worker log, screenshots, driver version, CUDA state, and any measured timings. Never record invented diagnoses or performance.

## Native Windows rebuild

Install stable Rust with MSVC target and Visual Studio Build Tools C++ workload. Run `powershell -File scripts/package-windows.ps1`. The script pins and verifies the isolated Python runtime, packages Pillow, runs headless application/worker/database smoke checks, and creates the ZIP.

No Node.js, React, TypeScript, Tauri, WebView, system Python, or HTTP server is used by this portable native package.

## Ubuntu dependencies

eframe development packages: `libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`. Legacy Tauri additionally needs `libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev`.
