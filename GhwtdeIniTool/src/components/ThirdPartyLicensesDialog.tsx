import { useEffect, useState } from "react";

import { loadThirdPartyLicenses } from "../services/thirdPartyLicensesApi";

type ThirdPartyLicensesDialogProps = {
  onClose: () => void;
};

export function ThirdPartyLicensesDialog({
  onClose,
}: ThirdPartyLicensesDialogProps) {
  const [licenses, setLicenses] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    let isCurrent = true;

    loadThirdPartyLicenses()
      .then((contents) => isCurrent && setLicenses(contents))
      .catch((err) => isCurrent && setError(String(err)));

    return () => {
      isCurrent = false;
    };
  }, []);

  return (
    <div className="settings-backdrop" role="presentation">
      <section
        className="settings-panel third-party-licenses-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="third-party-licenses-title"
      >
        <div className="settings-header">
          <h2 id="third-party-licenses-title">Third-party licenses</h2>
          <button
            className="close-btn"
            type="button"
            onClick={onClose}
            aria-label="Close third-party licenses"
          >
            &times;
          </button>
        </div>

        {error ? (
          <p className="settings-error">{error}</p>
        ) : licenses ? (
          <pre className="third-party-licenses-text">{licenses}</pre>
        ) : (
          <p className="third-party-licenses-loading">Loading licenses...</p>
        )}
      </section>
    </div>
  );
}
