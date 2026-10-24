import type { MemoryRecord, MemoryStore } from './memory-store';

/**
 * Reference in-process implementation of {@link MemoryStore}.
 * Useful for development and tests; state is lost on restart.
 */
export class InMemoryStore implements MemoryStore {
  private threads = new Map<string, Map<string, unknown>>();
  private logs = new Map<string, MemoryRecord[]>();

  get(threadId: string, key: string): Promise<unknown> {
    return Promise.resolve(this.threadOf(threadId).get(key));
  }

  set(threadId: string, key: string, value: unknown): Promise<void> {
    this.threadOf(threadId).set(key, value);
    this.appendLog(threadId, key, value);
    return Promise.resolve();
  }

  append(threadId: string, key: string, value: unknown): Promise<void> {
    const existing = this.threadOf(threadId).get(key);
    const next: unknown[] = Array.isArray(existing)
      ? [...(existing as unknown[]), value]
      : [value];
    this.threadOf(threadId).set(key, next);
    this.appendLog(threadId, key, next);
    return Promise.resolve();
  }

  history(threadId: string): Promise<MemoryRecord[]> {
    return Promise.resolve(this.logs.get(threadId) ?? []);
  }

  clear(threadId: string): Promise<void> {
    this.threads.delete(threadId);
    this.logs.delete(threadId);
    return Promise.resolve();
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
