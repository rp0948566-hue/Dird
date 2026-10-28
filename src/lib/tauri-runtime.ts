import { invoke, type InvokeArgs } from '@tauri-apps/api/core';

type TauriWindow = Window & {
  isTauri?: () => boolean;
  __TAURI_INTERNALS__?: { invoke?: unknown };
};

export function isTauriRuntime(): boolean {
  if (typeof window === 'undefined') return false;
  const runtime = window as TauriWindow;
  return typeof runtime.isTauri === 'function'
    ? runtime.isTauri()
    : typeof runtime.__TAURI_INTERNALS__?.invoke === 'function';
}

export function invokeTauri<T>(command: string, args?: InvokeArgs): Promise<T> {
  if (!isTauriRuntime()) {
    return Promise.reject(new Error('Tauri desktop backend unavailable. Launch DIRD+ as a desktop app.'));
  }
  return invoke<T>(command, args);
}
