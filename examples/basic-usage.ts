import { Orchestrator, BaseAgent, RemoteAgent } from '../src/index';
import type { Task } from '../src/index';

/** A local, in-process agent that plans a marketing task. */
class PlannerAgent extends BaseAgent {
  readonly name = 'planner';
  readonly capabilities = ['plan.marketing'];

  protected async handle(task: Task): Promise<unknown> {
    return { plan: `Launch a campaign for: ${task.payload as string}` };
  }
}

async function main() {
  const orchestrator = new Orchestrator();

  orchestrator.register(new PlannerAgent());

  // A polyglot agent — e.g. a Python service — plugged in over HTTP.
  orchestrator.register(
    new RemoteAgent({
      name: 'python-copywriter',
      capabilities: ['write.copy'],
      endpoint: 'http://localhost:8000/agents/copywriter',
    }),
  );

  orchestrator.events.on((event) => {
    console.log(`[${event.type}]`, event.type === 'task.completed' ? event.result : '');
  });

  orchestrator.submit({ capability: 'plan.marketing', payload: 'new product launch' });

  await orchestrator.run();
}

main().catch(console.error);
