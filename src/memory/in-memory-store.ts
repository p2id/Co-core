import type { MemoryRecord, MemoryStore } from './memory-store';

/**
 * Reference in-process implementation of {@link MemoryStore}.
 * Useful for development and tests; state is lost on restart.
 */
export class InMemoryStore implements MemoryStore {
  private threads = new Map<string, Map<string, unknown>>();
  private logs = new Map<string, MemoryRecord[]>();

  async get(threadId: string, key: string): Promise<unknown | undefined> {
    return this.threadOf(threadId).get(key);
  }

  async set(threadId: string, key: string, value: unknown): Promise<void> {
    this.threadOf(threadId).set(key, value);
    this.appendLog(threadId, key, value);
  }

  async append(threadId: string, key: string, value: unknown): Promise<void> {
    const existing = this.threadOf(threadId).get(key);
    const next = Array.isArray(existing) ? [...existing, value] : [value];
    this.threadOf(threadId).set(key, next);
    this.appendLog(threadId, key, next);
  }

  async history(threadId: string): Promise<MemoryRecord[]> {
    return this.logs.get(threadId) ?? [];
  }

  async clear(threadId: string): Promise<void> {
    this.threads.delete(threadId);
    this.logs.delete(threadId);
  }

  private threadOf(threadId: string): Map<string, unknown> {
    let thread = this.threads.get(threadId);
    if (!thread) {
      thread = new Map();
      this.threads.set(threadId, thread);
    }
    return thread;
  }

  private appendLog(threadId: string, key: string, value: unknown): void {
    const entries = this.logs.get(threadId) ?? [];
    entries.push({ key, value, updatedAt: Date.now() });
    this.logs.set(threadId, entries);
  }
}
