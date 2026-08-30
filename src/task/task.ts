export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface TaskInput {
  /** Capability required to execute this task; used for agent routing. */
  capability: string;
  /** Arbitrary payload passed to the agent that handles this task. */
  payload: unknown;
  /** Higher values run first. Defaults to 0. */
  priority?: number;
  /** Memory thread that groups related tasks. Auto-generated if omitted. */
  threadId?: string;
}

export interface Task extends TaskInput {
  readonly id: string;
  status: TaskStatus;
  readonly createdAt: number;
  readonly priority: number;
  readonly threadId: string;
}

export function createTask(input: TaskInput): Task {
  return {
    id: crypto.randomUUID(),
    status: 'pending',
    createdAt: Date.now(),
    priority: input.priority ?? 0,
    capability: input.capability,
    payload: input.payload,
    threadId: input.threadId ?? crypto.randomUUID(),
  };
}
