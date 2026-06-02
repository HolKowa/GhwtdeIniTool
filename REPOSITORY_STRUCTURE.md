# Repository Structure

This repository currently contains a basic Tauri 2 desktop application in the
`GhwtdeIniTool/` directory, plus repository-level CI configuration.

## Top Level

```text
.
|-- .github/
|   `-- workflows/
|       `-- build.yml
|-- GhwtdeIniTool/
|   |-- src/
|   |-- src-tauri/
|   |-- public/
|   |-- package.json
|   |-- pnpm-lock.yaml
|   |-- pnpm-workspace.yaml
|   |-- vite.config.ts
|   |-- tsconfig.json
|   |-- tsconfig.node.json
|   |-- index.html
|   `-- README.md
`-- LICENSE
```

Generated or dependency directories such as `node_modules/`, `dist/`, and
`src-tauri/target/` may also exist locally, but they are build artifacts rather
than source structure.

## Frontend

The frontend is a React 19 and TypeScript app built with Vite.

Important files:

- `GhwtdeIniTool/src/main.tsx` mounts the React application.
- `GhwtdeIniTool/src/App.tsx` contains the current UI.
- `GhwtdeIniTool/src/App.css` contains the current app styling.
- `GhwtdeIniTool/vite.config.ts` configures Vite for Tauri development on port
  `1420`.

Current frontend behavior:

- Shows the `GhwtdeIniTool` welcome screen.
- Checks for app updates through `@tauri-apps/plugin-updater`.
- Can download, install, skip, or report errors for updates.
- Relaunches the app through `@tauri-apps/plugin-process` after installing an
  update.

Frontend scripts from `GhwtdeIniTool/package.json`:

- `pnpm dev` starts the Vite development server.
- `pnpm build` runs TypeScript checking and builds the frontend.
- `pnpm preview` previews the built frontend.
- `pnpm tauri` runs the Tauri CLI.

## Backend

The backend is the Rust side of the Tauri app in `GhwtdeIniTool/src-tauri/`.

Important files:

- `GhwtdeIniTool/src-tauri/src/main.rs` starts the Tauri binary and calls the
  library entry point.
- `GhwtdeIniTool/src-tauri/src/lib.rs` configures the Tauri builder, plugins,
  and command handlers.
- `GhwtdeIniTool/src-tauri/Cargo.toml` defines Rust dependencies and crate
  metadata.
- `GhwtdeIniTool/src-tauri/tauri.conf.json` defines app metadata, windows,
  bundling, updater settings, and frontend build integration.
- `GhwtdeIniTool/src-tauri/capabilities/default.json` defines default Tauri
  permissions.

Current backend behavior:

- Registers the Tauri opener, updater, and process plugins.
- Exposes a starter `greet` command from Rust.
- Uses `tauri.conf.json` to configure bundling and updater artifacts.

## Build And Release

The repository has one GitHub Actions workflow:

- `.github/workflows/build.yml`

Current workflow behavior:

- Builds Windows artifacts on pushes to `main` and `integration`.
- Can optionally build Linux artifacts through manual workflow dispatch.
- Uses Node.js 24, pnpm, and Rust.
- Builds the Tauri app with signing secrets.
- Uploads installer and portable artifacts.
- Creates a GitHub release when pushing to `main`.

## Tests

No dedicated test setup is present at the moment.

What was checked:

- No `test` script exists in `GhwtdeIniTool/package.json`.
- No frontend test framework dependency such as Vitest, Jest, Playwright, or
  Testing Library is listed.
- No `*.test.*` or `*.spec.*` files were found.
- No Rust `#[test]` functions were found.

Useful current validation commands:

```sh
cd GhwtdeIniTool
pnpm build
cd src-tauri
cargo check
```

These are build and compile checks, not a replacement for unit or integration
tests.
