# Contributing to Atlas Core

Thanks for considering a contribution. This project stays small and strict on purpose — please read this before opening a pull request.

## Ground rules

- **English only.** All code, comments, commit messages, and documentation.
- **No dead code.** If you replace something, delete the old path — don't comment it out.
- **One responsibility per file.** If a file is doing two jobs, split it.
- **Interfaces over implementations.** The core depends on `Agent`, `MemoryStore`, and `EventBus` as interfaces. Concrete implementations (Redis store, HTTP server, specific agents) belong in separate packages, not in `src/`.
- **Every public export needs a test.** Run `npm test` before submitting.
- **Strict TypeScript.** `npm run typecheck` must pass with no `any` unless justified in a comment.

## Getting started

```bash
git clone https://github.com/<org>/atlas-core.git
cd atlas-core
npm install
npm test
```

## Workflow

1. Open an issue first for anything beyond a small fix, so the design can be discussed before code is written.
2. Keep pull requests focused — one concern per PR.
3. Add or update tests alongside any behavior change.
4. Run `npm run lint` and `npm run typecheck` before pushing.

## Adding a new agent or memory backend

These live outside the core package (e.g. `@atlas/memory-redis`, `@atlas/agent-slack`). Implement the relevant interface (`Agent` or `MemoryStore`) exactly as defined in `src/`, and open a PR against the community packages index once it's ready — the core itself should not grow new dependencies to support one backend.

## Reporting bugs

Open an issue with: what you expected, what happened instead, and a minimal reproduction if possible.
