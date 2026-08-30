import { AgentRegistry } from '../agent/agent-registry';
import type { Agent } from '../agent/agent';
import { TaskQueue } from '../task/task-queue';
import { createTask } from '../task/task';
import type { Task, TaskInput } from '../task/task';
import { EventBus } from '../events/event-bus';
import type { MemoryStore } from '../memory/memory-store';
import { InMemoryStore } from '../memory/in-memory-store';
import { NoAgentForCapabilityError } from './errors';

export interface OrchestratorOptions {
  /** Backing store for memory threads. Defaults to an in-process store. */
  memory?: MemoryStore;
}

/**
 * The core execution engine. It queues tasks, routes each one to the
 * agent registered for its capability, and reports progress through
 * the event bus so any dashboard or logger can observe activity live.
 */
export class Orchestrator {
  readonly agents = new AgentRegistry();
  readonly events = new EventBus();
  readonly memory: MemoryStore;

  private queue = new TaskQueue();
  private running = false;

  constructor(options: OrchestratorOptions = {}) {
    this.memory = options.memory ?? new InMemoryStore();
  }

  register(agent: Agent): void {
    this.agents.register(agent);
  }

  /** Queues a task for execution and returns its record immediately. */
  submit(input: TaskInput): Task {
    const task = createTask(input);
    this.queue.enqueue(task);
    this.events.emit({ type: 'task.queued', task });
    return task;
  }

  /** Drains the queue, executing tasks one at a time in priority order. */
  async run(): Promise<void> {
    if (this.running) return;
    this.running = true;

    try {
      while (!this.queue.isEmpty()) {
        const task = this.queue.dequeue();
        if (task) await this.execute(task);
      }
    } finally {
      this.running = false;
    }
  }

  private async execute(task: Task): Promise<void> {
    const agent = this.agents.resolve(task.capability);
    if (!agent) {
      const error = new NoAgentForCapabilityError(task.capability);
      task.status = 'failed';
      this.events.emit({ type: 'task.failed', task, agent: 'none', error: error.message });
      return;
    }

    task.status = 'running';
    this.events.emit({ type: 'task.started', task, agent: agent.name });

    const result = await agent.execute(task, { threadId: task.threadId, memory: this.memory });

    if (result.success) {
      task.status = 'completed';
      this.events.emit({ type: 'task.completed', task, agent: agent.name, result: result.output });
    } else {
      task.status = 'failed';
      this.events.emit({
        type: 'task.failed',
        task,
        agent: agent.name,
        error: result.error ?? 'Unknown error',
      });
    }
  }
}
