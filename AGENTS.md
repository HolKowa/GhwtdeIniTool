# AGENTS.md

Repo-scoped instructions for Codex and other coding agents working here.
These complement user requests and project-specific context; use judgment for
trivial tasks.

## 1. Think Before Coding

- Read the relevant files before changing code.
- State important assumptions when they affect the implementation.
- If a request has multiple plausible meanings, ask or briefly name the chosen
  interpretation before making a risky change.
- Push back when a request would make the app more fragile, harder to maintain,
  or inconsistent with the existing project.

## 2. Keep It Simple

- Implement the smallest change that solves the user's actual request.
- Do not add speculative features, abstractions, configuration, or broad error
  handling that was not requested.
- Prefer established project patterns over new architecture.
- If a solution starts growing large, reassess whether a simpler implementation
  would do the job.

## 3. Make Surgical Changes

- Touch only files that are directly related to the request.
- Match the style already used in the surrounding TypeScript, React, CSS, Rust,
  and config files.
- Do not refactor, reformat, or clean up unrelated code.
- Remove imports, variables, functions, or assets made unused by your own
  changes, but leave pre-existing unrelated cleanup for a separate request.
- Mention unrelated issues you notice instead of quietly changing them.

## 4. Prefer SOLID Design

- Use SOLID as practical guidance, not as a reason to over-abstract small code.
- Keep modules, components, hooks, commands, and services focused on one clear
  responsibility.
- Prefer extending behavior by adding focused collaborators over editing large
  multi-purpose functions or components.
- Keep UI, state orchestration, external API calls, parsing, persistence, and
  platform-specific behavior separated when that separation makes the code
  easier to read, test, or change.
- Depend on narrow interfaces or small wrapper functions around external APIs
  when direct coupling would make behavior hard to validate.
- Avoid abstractions that only rename existing code or add indirection without
  reducing meaningful duplication, risk, or complexity.

## 5. Work Toward Verifiable Results

- For fixes, prefer reproducing or identifying the failing behavior first.
- For new behavior, define what success looks like before implementing.
- Validate the relevant surface after changes. Useful commands include:

```sh
cd GhwtdeIniTool
pnpm build
cd src-tauri
cargo check
```

- If a validation command cannot be run, explain why and name the residual risk.
- After a change is implemented and the relevant validation passes, update
  `REPOSITORY_STRUCTURE.md` when the change affects app structure, important
  files, behavior, validation commands, or project workflow.

## 6. Project Context

- The main app lives in `GhwtdeIniTool/`.
- The frontend is React, TypeScript, and Vite.
- The desktop backend is Tauri 2 with Rust in `GhwtdeIniTool/src-tauri/`.
- There is currently no dedicated test suite; build and compile checks are the
  primary validation commands.
- Keep repository-root documentation and CI changes separate from app changes
  unless the user asks for both.
