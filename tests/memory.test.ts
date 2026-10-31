import { describe, expect, it } from 'vitest';
import { InMemoryStore } from '../src/memory/in-memory-store';

describe('InMemoryStore', () => {
  it('stores and retrieves values scoped to a thread', async () => {
    const store = new InMemoryStore();
    await store.set('thread-1', 'company', 'Acme Inc.');

    expect(await store.get('thread-1', 'company')).toBe('Acme Inc.');
    expect(await store.get('thread-2', 'company')).toBeUndefined();
  });

  it('accumulates values with append', async () => {
    const store = new InMemoryStore();
    await store.append('thread-1', 'log', 'step one');
    await store.append('thread-1', 'log', 'step two');

    expect(await store.get('thread-1', 'log')).toEqual(['step one', 'step two']);
  });

  it('records history for a thread', async () => {
    const store = new InMemoryStore();
    await store.set('thread-1', 'status', 'started');
    await store.set('thread-1', 'status', 'done');

    const history = await store.history('thread-1');
    expect(history).toHaveLength(2);
    expect(history[1].value).toBe('done');
  });

  it('returns history snapshots that cannot mutate the stored log', async () => {
    const store = new InMemoryStore();
    await store.set('thread-1', 'status', 'started');
    await store.set('thread-1', 'status', 'done');

    const history = await store.history('thread-1');
    history.pop();
    history[0].key = 'tampered';
    history[0].updatedAt = 0;

    const nextHistory = await store.history('thread-1');
    expect(nextHistory).toHaveLength(2);
    expect(nextHistory[0]).toMatchObject({
      key: 'status',
      value: 'started',
    });
    expect(nextHistory[0].updatedAt).not.toBe(0);
  });

  it('clears a thread completely', async () => {
    const store = new InMemoryStore();
    await store.set('thread-1', 'status', 'started');
    await store.clear('thread-1');

    expect(await store.get('thread-1', 'status')).toBeUndefined();
    expect(await store.history('thread-1')).toEqual([]);
  });
});
