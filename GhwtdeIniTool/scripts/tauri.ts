import { execFileSync, spawn } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const generatedConfigPath = join(
  root,
  "src-tauri",
  "tauri.updater.generated.json"
);

loadEnvFile(join(root, ".env"));
loadEnvFile(join(root, ".env.local"));
prepareReleaseLicenses();
const hasGeneratedConfig = writeUpdaterConfig(process.argv[2]);
runTauri(hasGeneratedConfig);

function prepareReleaseLicenses() {
  if (process.argv[2] !== "build") {
    return;
  }

  execFileSync(
    "pnpm",
    ["licenses:generate"],
    {
      cwd: root,
      // Windows exposes pnpm as a .cmd shim, which execFileSync cannot run directly.
      shell: process.platform === "win32",
      stdio: "inherit",
    },
  );
  process.env.GHWTDE_INCLUDE_THIRD_PARTY_LICENSES = "1";
}

function loadEnvFile(path: string) {
  if (!existsSync(path)) {
    return;
  }

  for (const line of readFileSync(path, "utf8").split(/\r?\n/)) {
    const match = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)?\s*$/);

    if (!match || process.env[match[1]] !== undefined) {
      continue;
    }

    process.env[match[1]] = stripEnvValue(match[2] ?? "");
  }
}

function stripEnvValue(value: string) {
  const trimmed = value.trim();

  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }

  return trimmed.replace(/\s+#.*$/, "");
}

function writeUpdaterConfig(command: string | undefined) {
  const repository =
    process.env.GHWTDE_UPDATER_REPOSITORY?.trim() ||
    process.env.GITHUB_REPOSITORY?.trim();
  const endpoint = process.env.GHWTDE_UPDATER_ENDPOINT?.trim();
  const appVersion =
    command === "dev" ? process.env.GHWTDE_APP_VERSION?.trim() : undefined;

  if (!endpoint && !repository) {
    return false;
  }

  const config = {
    ...(appVersion ? { version: appVersion } : {}),
    plugins: {
      updater: {
        endpoints: [
          endpoint ||
            `https://github.com/${repository}/releases/latest/download/latest.json`,
        ],
      },
    },
  };

  writeFileSync(generatedConfigPath, `${JSON.stringify(config, null, 2)}\n`);
  return true;
}

function runTauri(hasGeneratedConfig: boolean) {
  const [command, ...args] = process.argv.slice(2);
  const shouldUseGeneratedConfig =
    hasGeneratedConfig && command && !command.startsWith("-");
  const tauriArgs = shouldUseGeneratedConfig
    ? [command, "--config", generatedConfigPath, ...args]
    : process.argv.slice(2);
  const tauriBin = join(
    root,
    "node_modules",
    ".bin",
    process.platform === "win32" ? "tauri.cmd" : "tauri"
  );

  const child = spawn(tauriBin, tauriArgs, {
    cwd: root,
    env: process.env,
    shell: process.platform === "win32",
    stdio: "inherit",
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }

    process.exit(code ?? 0);
  });
}
