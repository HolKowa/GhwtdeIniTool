/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_GITHUB_UPDATER_TOKEN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
