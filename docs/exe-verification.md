# Executable verification record

Date: 2026-10-27. Host: Ubuntu x86-64.

## Passed

- Native Rust release build.
- Self-contained Ubuntu package headless smoke: Python worker health, encrypted database round trip, embedded 256x256 icon.
- Windows cross-build: `PE32+`, x86-64, Windows GUI subsystem, embedded `.rsrc`.
- Portable Windows package contains executable, isolated Python runtime, Pillow, worker source, model guidance, license, checksum, and `test-windows.ps1`.

## GUI display diagnosis

`xvfb-run` failed because Xorg requires `/tmp/.X11-unix` owner `root`, but this host reports `nobody:nogroup`; passwordless correction was unavailable. A real Wayland launch was then attempted outside the sandbox. The process launched, but `--gui-smoke-test` did not emit its report before a 25-second timeout. This is not recorded as a GUI pass.

## Prohibited claims

No Windows execution, CUDA, RTX 5070, model accuracy, clinical output, or performance result has been validated. Use `windows-build.md` after reboot.
