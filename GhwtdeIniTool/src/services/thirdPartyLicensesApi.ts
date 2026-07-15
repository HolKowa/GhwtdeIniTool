import { invoke } from "@tauri-apps/api/core";

export function loadThirdPartyLicenses() {
  return invoke<string>("third_party_licenses");
}
