import { afterEach, describe, expect, it, vi } from 'vitest';
import { invokeTauri, isTauriRuntime } from './tauri-runtime';

vi.mock('@/lib/db-sql/client', () => ({
  dbOpen: vi.fn(),
  dbClose: vi.fn(),
  dbExists: vi.fn(),
  dbIsOpen: vi.fn(),
  dbChangePassword: vi.fn(),
}));

import { dbExists, dbIsOpen } from '@/lib/db-sql/client';
import { useAuthStore } from '@/stores/auth-store';

const runtimeWindow = window as Window & {
  isTauri?: () => boolean;
  __TAURI_INTERNALS__?: { invoke?: unknown };
};

afterEach(() => {
  delete runtimeWindow.isTauri;
  delete runtimeWindow.__TAURI_INTERNALS__;
  vi.clearAllMocks();
});

describe('Tauri runtime guard', () => {
  it('recognizes the desktop runtime', () => {
    runtimeWindow.isTauri = () => true;
    expect(isTauriRuntime()).toBe(true);
  });

  it('recognizes the injected IPC bridge when the helper is absent', () => {
    runtimeWindow.__TAURI_INTERNALS__ = { invoke: () => undefined };
    expect(isTauriRuntime()).toBe(true);
  });

  it('rejects IPC in a regular browser with an actionable error', async () => {
    expect(isTauriRuntime()).toBe(false);
    await expect(invokeTauri('db_is_open')).rejects.toThrow(
      'Tauri desktop backend unavailable. Launch DIRD+ as a desktop app.',
    );
  });

  it('does not try to unlock or inspect patient storage from a browser', async () => {
    await useAuthStore.getState().checkBoot();
    expect(useAuthStore.getState().status).toBe('unavailable');
    expect(dbIsOpen).not.toHaveBeenCalled();
    expect(dbExists).not.toHaveBeenCalled();
  });

  it('keeps the existing native first-run database check', async () => {
    runtimeWindow.isTauri = () => true;
    vi.mocked(dbIsOpen).mockResolvedValue(false);
    vi.mocked(dbExists).mockResolvedValue(false);
    await useAuthStore.getState().checkBoot();
    expect(useAuthStore.getState().status).toBe('first-run');
    expect(dbIsOpen).toHaveBeenCalledOnce();
    expect(dbExists).toHaveBeenCalledOnce();
  });
});
