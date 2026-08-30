export class NoAgentForCapabilityError extends Error {
  constructor(capability: string) {
    super(`No agent registered for capability "${capability}"`);
    this.name = 'NoAgentForCapabilityError';
  }
}
