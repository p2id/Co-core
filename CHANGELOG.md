# Changelog

All notable changes to this project are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - Unreleased

### Added
- `Orchestrator`: capability-based task routing, priority queue, lifecycle events.
- `Agent` interface, `BaseAgent` for in-process agents, `RemoteAgent` for polyglot HTTP agents.
- `AgentRegistry` with last-registered-wins capability resolution.
- `MemoryStore` interface and `InMemoryStore` reference implementation.
- `EventBus` for typed task lifecycle events (`task.queued`, `task.started`, `task.completed`, `task.failed`).
- Wire protocol (`RemoteAgentRequest` / `RemoteAgentResponse`) for cross-language agents.
- Test suite covering the orchestrator, task queue, and memory store.
