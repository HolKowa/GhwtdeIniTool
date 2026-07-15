import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const projectDirectory = fileURLToPath(new URL("..", import.meta.url));
const rustDirectory = join(projectDirectory, "src-tauri");
const licenseFile = join(
  rustDirectory,
  "resources",
  "THIRD_PARTY_LICENSES.txt",
);
const newRockerLicenseFile = join(
  rustDirectory,
  "resources",
  "licenses",
  "NewRocker-OFL-1.1.txt",
);
const generatedMarker = "\n\n===============================================================================\nGenerated dependency notices\n===============================================================================\n";
const temporaryDirectory = mkdtempSync(join(tmpdir(), "ghwtdeinitool-licenses-"));
const frontendLicenses = join(temporaryDirectory, "frontend-licenses.txt");
const rustLicenses = join(temporaryDirectory, "rust-licenses.txt");

try {
  execFileSync(
    "pnpm",
    [
      "exec",
      "pnpm-licenses",
      "generate-disclaimer",
      "--prod",
      "--output-file",
      frontendLicenses,
    ],
    { cwd: projectDirectory, stdio: "inherit" },
  );
  execFileSync(
    process.env.CARGO_ABOUT ?? "cargo-about",
    [
      "generate",
      "--locked",
      "--fail",
      "--output-file",
      rustLicenses,
      "about.hbs",
    ],
    { cwd: rustDirectory, stdio: "inherit" },
  );

  const generatedLicenses = `${generatedMarker}\nFrontend production dependencies\n--------------------------------\n\n${readFileSync(frontendLicenses, "utf8").trim()}\n\nRust production dependencies\n----------------------------\n\n${readFileSync(rustLicenses, "utf8").trim()}\n`;
  const newRockerLicense = readFileSync(newRockerLicenseFile, "utf8").trim();
  writeFileSync(
    licenseFile,
    `Third-Party Licenses\n====================\n\n${newRockerLicense}${generatedLicenses}`,
    "utf8",
  );
} finally {
  rmSync(temporaryDirectory, { recursive: true, force: true });
}
