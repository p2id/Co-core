import { describe, expect, it } from 'vitest';
import { TaskQueue } from '../src/task/task-queue';
import { createTask } from '../src/task/task';

describe('TaskQueue', () => {
  it('dequeues in FIFO order when priorities are equal', () => {
    const queue = new TaskQueue();
    const first = createTask({ capability: 'noop', payload: 1 });
    const second = createTask({ capability: 'noop', payload: 2 });

    queue.enqueue(first);
    queue.enqueue(second);

    expect(queue.dequeue()?.id).toBe(first.id);
    expect(queue.dequeue()?.id).toBe(second.id);
  });

  it('dequeues higher priority tasks first', () => {
    const queue = new TaskQueue();
    const low = createTask({ capability: 'noop', payload: 'low', priority: 0 });
    const high = createTask({ capability: 'noop', payload: 'high', priority: 10 });

    queue.enqueue(low);
    queue.enqueue(high);

    expect(queue.dequeue()?.id).toBe(high.id);
    expect(queue.dequeue()?.id).toBe(low.id);
  });

  it('reports size and emptiness correctly', () => {
    const queue = new TaskQueue();
    expect(queue.isEmpty()).toBe(true);

    queue.enqueue(createTask({ capability: 'noop', payload: null }));
    expect(queue.size).toBe(1);
    expect(queue.isEmpty()).toBe(false);
  });
});
