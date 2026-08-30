import type { Task } from './task';

/**
 * Priority queue for tasks awaiting execution.
 * Higher `priority` values are dequeued first; ties preserve insertion order.
 */
export class TaskQueue {
  private items: Task[] = [];

  enqueue(task: Task): void {
    const index = this.items.findIndex((existing) => existing.priority < task.priority);
    if (index === -1) {
      this.items.push(task);
    } else {
      this.items.splice(index, 0, task);
    }
  }

  dequeue(): Task | undefined {
    return this.items.shift();
  }

  peek(): Task | undefined {
    return this.items[0];
  }

  get size(): number {
    return this.items.length;
  }

  isEmpty(): boolean {
    return this.items.length === 0;
  }
}
