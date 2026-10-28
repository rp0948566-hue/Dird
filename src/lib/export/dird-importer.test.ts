import { describe, expect, it } from 'vitest';
import { runAtomicImport, type ImportResult } from './dird-importer';

function transactionHarness() {
  let committed = false;
  let rolledBack = false;
  const transaction = async (work: () => Promise<void>) => {
    try {
      await work();
      committed = true;
    } catch (error) {
      rolledBack = true;
      throw error;
    }
  };
  return { transaction, committed: () => committed, rolledBack: () => rolledBack };
}

const result = (success: boolean): ImportResult => ({
  success,
  error: success ? undefined : 'invalid archive record',
  imagesImported: success ? 1 : 0,
  detectionsImported: 0,
  segmentationsImported: 0,
  measurementsImported: 0,
  reportsImported: 0,
});

describe('runAtomicImport', () => {
  it('commits only a successful archive import', async () => {
    const tx = transactionHarness();
    const imported = await runAtomicImport(tx.transaction, async () => result(true));
    expect(imported.success).toBe(true);
    expect(tx.committed()).toBe(true);
    expect(tx.rolledBack()).toBe(false);
  });

  it('turns a returned import failure into rollback', async () => {
    const tx = transactionHarness();
    await expect(runAtomicImport(tx.transaction, async () => result(false))).rejects.toThrow('invalid archive record');
    expect(tx.committed()).toBe(false);
    expect(tx.rolledBack()).toBe(true);
  });
});
