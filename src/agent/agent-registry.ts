import type { Agent } from './agent';

/**
 * Holds registered agents and resolves which one should handle a given
 * capability. Later registrations for the same capability take priority,
 * so a specialised agent can override a general-purpose default.
 */
export class AgentRegistry {
  private agents: Agent[] = [];

  register(agent: Agent): void {
    this.agents.push(agent);
  }

  unregister(name: string): void {
    this.agents = this.agents.filter((agent) => agent.name !== name);
  }

  resolve(capability: string): Agent | undefined {
    for (const agent of [...this.agents].reverse()) {
      if (agent.capabilities.includes(capability)) {
        return agent;
      }
    }
    return undefined;
  }

  list(): readonly Agent[] {
    return this.agents;
  }
}
