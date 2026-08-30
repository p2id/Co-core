import type { Task } from '../task/task';

export type OrchestratorEvent =
  | { type: 'task.queued'; task: Task }
  | { type: 'task.started'; task: Task; agent: string }
  | { type: 'task.completed'; task: Task; agent: string; result: unknown }
  | { type: 'task.failed'; task: Task; agent: string; error: string };

export type EventListener = (event: OrchestratorEvent) => void;
