import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";

const REPOSITORY_URL = "https://github.com/HolKowa/GhwtdeIniLoader";

function openRepository() {
  if (isTauri()) {
    void openUrl(REPOSITORY_URL);
    return;
  }

  window.open(REPOSITORY_URL, "_blank", "noopener,noreferrer");
}

type AboutDialogProps = {
  onClose: () => void;
  onOpenThirdPartyLicenses: () => void;
};

export function AboutDialog({
  onClose,
  onOpenThirdPartyLicenses,
}: AboutDialogProps) {
  const [version, setVersion] = useState("");

  useEffect(() => {
    let isCurrent = true;

    getVersion()
      .then((appVersion) => {
        if (isCurrent) {
          setVersion(appVersion);
        }
      })
      .catch(() => {
        if (isCurrent) {
          setVersion("Development browser");
        }
      });

    return () => {
      isCurrent = false;
    };
  }, []);

  return (
    <div className="settings-backdrop" role="presentation">
      <section
        className="settings-panel about-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="about-title"
      >
        <div className="settings-header">
          <h2 id="about-title">About</h2>
          <button
            className="close-btn"
            type="button"
            onClick={onClose}
            aria-label="Close about"
          >
            &times;
          </button>
        </div>

        <section className="about-section" aria-labelledby="about-app-title">
          <h3 id="about-app-title">GhwtdeIniTool</h3>
          <p>Version {version || "Loading..."}</p>
        </section>

        <section className="about-section" aria-labelledby="about-support-title">
          <h3 id="about-support-title">Support</h3>
          <button
            className="about-link"
            type="button"
            onClick={openRepository}
          >
            GitHub repository / Report a bug
          </button>
        </section>

        <section className="about-section" aria-labelledby="about-legal-title">
          <h3 id="about-legal-title">Legal</h3>
          <button
            className="about-link"
            type="button"
            onClick={onOpenThirdPartyLicenses}
          >
            Third-party licenses
          </button>
        </section>
      </section>
    </div>
  );
}
