import type { Task } from '../task/task';
import type { MemoryStore } from '../memory/memory-store';

export interface AgentContext {
  threadId: string;
  memory: MemoryStore;
}

export interface AgentResult {
  success: boolean;
  output?: unknown;
  error?: string;
}

/**
 * Contract every executor in the system must satisfy, whether it runs
 * in-process or as a remote service written in another language.
 */
export interface Agent {
  /** Unique, human-readable identifier used in logs and routing. */
  readonly name: string;
  /** Capabilities this agent can fulfil; matched against task.capability. */
  readonly capabilities: string[];
  execute(task: Task, context: AgentContext): Promise<AgentResult>;
}
