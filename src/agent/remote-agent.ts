import type { Agent, AgentContext, AgentResult } from './agent';
import type { Task } from '../task/task';
import type { RemoteAgentRequest, RemoteAgentResponse } from '../protocol/agent-protocol';

export interface RemoteAgentOptions {
  name: string;
  capabilities: string[];
  /** HTTP endpoint that implements the remote agent protocol. */
  endpoint: string;
  fetchImpl?: typeof fetch;
  timeoutMs?: number;
}

/**
 * Bridges a task to an agent process running outside this codebase —
 * Python, Go, or any language that speaks the JSON-over-HTTP contract
 * defined in {@link RemoteAgentRequest}. This is how a single core can
 * orchestrate a genuinely polyglot fleet of agents.
 */
export class RemoteAgent implements Agent {
  readonly name: string;
  readonly capabilities: string[];
  private readonly endpoint: string;
  private readonly fetchImpl: typeof fetch;
  private readonly timeoutMs: number;

  constructor(options: RemoteAgentOptions) {
    this.name = options.name;
    this.capabilities = options.capabilities;
    this.endpoint = options.endpoint;
    this.fetchImpl = options.fetchImpl ?? fetch;
    this.timeoutMs = options.timeoutMs ?? 30_000;
  }

  async execute(task: Task, context: AgentContext): Promise<AgentResult> {
    const request: RemoteAgentRequest = {
      taskId: task.id,
      capability: task.capability,
      payload: task.payload,
      threadId: context.threadId,
    };

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.timeoutMs);

    try {
      const response = await this.fetchImpl(this.endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
        signal: controller.signal,
      });

      if (!response.ok) {
        return { success: false, error: `Remote agent responded with HTTP ${response.status}` };
      }

      const result = (await response.json()) as RemoteAgentResponse;
      return { success: result.success, output: result.output, error: result.error };
    } catch (error) {
      return { success: false, error: (error as Error).message };
    } finally {
      clearTimeout(timeout);
    }
  }
}
