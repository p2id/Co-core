import type { EventListener, OrchestratorEvent } from './events';

/**
 * Minimal typed publish/subscribe channel used to observe orchestration
 * activity in real time — task lifecycle, agent execution, failures.
 * Power a live dashboard, a logger, or both from the same stream.
 */
export class EventBus {
  private listeners = new Set<EventListener>();

  /** Subscribes to all events. Returns an unsubscribe function. */
  on(listener: EventListener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  emit(event: OrchestratorEvent): void {
    for (const listener of this.listeners) {
      listener(event);
    }
  }
}
