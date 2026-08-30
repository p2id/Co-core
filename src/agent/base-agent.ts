import type { Agent, AgentContext, AgentResult } from './agent';
import type { Task } from '../task/task';

/**
 * Convenience base class for in-process agents. Implement {@link handle}
 * and return your output directly — success/failure shaping and error
 * capture are handled for you.
 */
export abstract class BaseAgent implements Agent {
  abstract readonly name: string;
  abstract readonly capabilities: string[];

  async execute(task: Task, context: AgentContext): Promise<AgentResult> {
    try {
      const output = await this.handle(task, context);
      return { success: true, output };
    } catch (error) {
      return { success: false, error: (error as Error).message };
    }
  }

  protected abstract handle(task: Task, context: AgentContext): Promise<unknown>;
}
