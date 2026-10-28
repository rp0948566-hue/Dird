# Browser unlock failure: removal plan

## Diagnosis

The screenshot is from `localhost:5173`, a normal Vite browser session. `useAuthStore.checkBoot()` calls `dbIsOpen()` and `dbExists()` unconditionally. Those helpers call Tauri `invoke`, which has no native IPC bridge in a regular browser. The resulting `TypeError` is stored as the auth error and shown under the password field.

## Change

1. Add a shared Tauri-runtime check and guarded invoke helper.
2. Make auth boot stop before database calls outside Tauri; render a clear desktop-runtime-required screen instead of the password form.
3. Route database and crypto IPC through the guard so other browser paths fail with an actionable error, never an undefined-property TypeError.
4. Add unit checks for runtime detection and guarded invoke; run tests and TypeScript verification.

## Safety boundary

Do not create a browser database fallback or store patient data in IndexedDB. The native desktop database and login behavior remain unchanged. Browser preview remains available only as a non-clinical shell until a native runtime is present.
