import { describe, expect, it } from 'vitest';
import { Orchestrator } from '../src/orchestrator/orchestrator';
import { BaseAgent } from '../src/agent/base-agent';
import type { Task } from '../src/task/task';
import type { AgentContext } from '../src/agent/agent';

class EchoAgent extends BaseAgent {
  readonly name = 'echo-agent';
  readonly capabilities = ['echo'];

  protected async handle(task: Task): Promise<unknown> {
    return `echo:${task.payload as string}`;
  }
}

class FailingAgent extends BaseAgent {
  readonly name = 'failing-agent';
  readonly capabilities = ['fail'];

  protected async handle(): Promise<unknown> {
    throw new Error('boom');
  }
}

class MemoryWritingAgent extends BaseAgent {
  readonly name = 'memory-writer';
  readonly capabilities = ['remember'];

  protected async handle(task: Task, context: AgentContext): Promise<unknown> {
    await context.memory.set(context.threadId, 'seen', task.payload);
    return 'stored';
  }
}

describe('Orchestrator', () => {
  it('routes a task to the agent that declares its capability', async () => {
    const orchestrator = new Orchestrator();
    orchestrator.register(new EchoAgent());

    const events: string[] = [];
    orchestrator.events.on((event) => events.push(event.type));

    orchestrator.submit({ capability: 'echo', payload: 'hello' });
    await orchestrator.run();

    expect(events).toEqual(['task.queued', 'task.started', 'task.completed']);
  });

  it('fails the task when no agent covers the capability', async () => {
    const orchestrator = new Orchestrator();
    const events: string[] = [];
    orchestrator.events.on((event) => events.push(event.type));

    orchestrator.submit({ capability: 'unknown', payload: null });
    await orchestrator.run();

    expect(events).toEqual(['task.queued', 'task.failed']);
  });

  it('captures agent exceptions as task failures instead of throwing', async () => {
    const orchestrator = new Orchestrator();
    orchestrator.register(new FailingAgent());

    let failure: string | undefined;
    orchestrator.events.on((event) => {
      if (event.type === 'task.failed') failure = event.error;
    });

    orchestrator.submit({ capability: 'fail', payload: null });
    await orchestrator.run();

    expect(failure).toBe('boom');
  });

  it('marks thrown agent executions as failed and continues draining the queue', async () => {
    const orchestrator = new Orchestrator();
    orchestrator.register({
      name: 'remote-agent',
      capabilities: ['remote'],
      execute: async () => {
        throw new Error('remote unavailable');
      },
    });
    orchestrator.register(new EchoAgent());

    const outcomes: string[] = [];
    orchestrator.events.on((event) => {
      if (event.type === 'task.failed') outcomes.push(`failed:${event.error}`);
      if (event.type === 'task.completed') outcomes.push(`completed:${event.agent}`);
    });

    const failedTask = orchestrator.submit({ capability: 'remote', payload: null });
    const nextTask = orchestrator.submit({ capability: 'echo', payload: 'still running' });
    await orchestrator.run();

    expect(failedTask.status).toBe('failed');
    expect(nextTask.status).toBe('completed');
    expect(outcomes).toEqual(['failed:remote unavailable', 'completed:echo-agent']);
  });

  it('normalizes non-Error rejection values in task failure events', async () => {
    const orchestrator = new Orchestrator();
    orchestrator.register({
      name: 'remote-agent',
      capabilities: ['remote'],
      execute: async () => {
        throw 'remote unavailable';
      },
    });

    let failure: string | undefined;
    orchestrator.events.on((event) => {
      if (event.type === 'task.failed') failure = event.error;
    });

    const task = orchestrator.submit({ capability: 'remote', payload: null });
    await orchestrator.run();

    expect(task.status).toBe('failed');
    expect(failure).toBe('remote unavailable');
  });

  it('gives agents access to a shared memory thread', async () => {
    const orchestrator = new Orchestrator();
    orchestrator.register(new MemoryWritingAgent());

    const task = orchestrator.submit({ capability: 'remember', payload: 'context-value' });
    await orchestrator.run();

    expect(await orchestrator.memory.get(task.threadId, 'seen')).toBe('context-value');
  });
});
