import { ExperimentalDisclaimerText } from "./ExperimentalDisclaimerText";

type ExperimentalWarningDialogProps = {
  onClose: () => void;
};

export function ExperimentalWarningDialog({
  onClose,
}: ExperimentalWarningDialogProps) {
  return (
    <div className="settings-backdrop" role="presentation">
      <section
        className="settings-panel experimental-warning-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="experimental-warning-title"
      >
        <div className="settings-header">
          <h2 id="experimental-warning-title">Experimental software warning</h2>
          <button
            className="close-btn"
            type="button"
            onClick={onClose}
            aria-label="Close experimental software warning"
          >
            &times;
          </button>
        </div>

        <div className="disclaimer-section">
          <ExperimentalDisclaimerText />
        </div>
      </section>
    </div>
  );
}
