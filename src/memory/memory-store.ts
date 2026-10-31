export interface MemoryRecord {
  key: string;
  value: unknown;
  updatedAt: number;
}

/**
 * Storage contract for persistent agent memory threads. Implement this
 * to back memory with Redis, Postgres, a vector database, or anything
 * else — the orchestrator only ever depends on this interface.
 */
export interface MemoryStore {
  get(threadId: string, key: string): Promise<unknown>;
  set(threadId: string, key: string, value: unknown): Promise<void>;
  append(threadId: string, key: string, value: unknown): Promise<void>;
  history(threadId: string): Promise<MemoryRecord[]>;
  clear(threadId: string): Promise<void>;
}
