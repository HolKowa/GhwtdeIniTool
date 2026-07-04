import { useState } from "react";

import type { InstrumentAnalyzeStatus } from "../hooks/useModsScanner";
import type {
  InstrumentAnalyzeMode,
  InstrumentAnalyzeProgress,
  InstrumentAnalyzeResult,
} from "../types/scanMods";

type InstrumentAnalyzeWizardProps = {
  error: string;
  onAnalyze: (mode: InstrumentAnalyzeMode) => void;
  onCancel: () => void;
  onClose: () => void;
  progress: InstrumentAnalyzeProgress | null;
  result: InstrumentAnalyzeResult | null;
  status: InstrumentAnalyzeStatus;
};

const modeOptions: Array<{
  label: string;
  mode: InstrumentAnalyzeMode;
  summary: string;
}> = [
  {
    label: "Read only missing",
    mode: "missing",
    summary: "Missing, stale, or invalid sidecar files",
  },
  {
    label: "Reread only songs with errors",
    mode: "errors",
    summary: "Fresh sidecar files currently showing Error",
  },
  {
    label: "Reread all",
    mode: "all",
    summary: "Every scanned song",
  },
];

export function InstrumentAnalyzeWizard({
  error,
  onAnalyze,
  onCancel,
  onClose,
  progress,
  result,
  status,
}: InstrumentAnalyzeWizardProps) {
  const [mode, setMode] = useState<InstrumentAnalyzeMode>("missing");
  const isAnalyzing = status === "analyzing";
  const isComplete = status === "complete";
  const isError = status === "error";
  const title = isComplete
    ? "Instrument analysis complete"
    : isError
      ? "Instrument analysis failed"
      : "Analyze instruments";
  const stepLabel = isComplete || isError
    ? "Step 3 of 3"
    : isAnalyzing
      ? "Step 2 of 3"
      : "Step 1 of 3";

  return (
    <div className="scan-wizard-backdrop" role="presentation">
      <section
        className="scan-wizard-panel instrument-analyze-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="instrument-analyze-title"
      >
        <div className="scan-wizard-header">
          <div>
            <p className="scan-wizard-step">{stepLabel}</p>
            <h2 id="instrument-analyze-title">{title}</h2>
          </div>
          <button
            className="close-btn"
            type="button"
            onClick={isComplete ? onClose : onCancel}
            aria-label="Close instrument analysis"
            disabled={isAnalyzing}
          >
            &times;
          </button>
        </div>

        <div className="scan-wizard-body">
          {!isAnalyzing && !isComplete && !isError && (
            <div className="instrument-mode-list">
              {modeOptions.map((option) => (
                <label className="instrument-mode-option" key={option.mode}>
                  <input
                    type="radio"
                    name="instrument-analyze-mode"
                    value={option.mode}
                    checked={mode === option.mode}
                    onChange={() => setMode(option.mode)}
                  />
                  <span>
                    <strong>{option.label}</strong>
                    <small>{option.summary}</small>
                  </span>
                </label>
              ))}
            </div>
          )}

          {isAnalyzing && (
            <>
              <p className="scan-wizard-summary">
                {progress
                  ? `Reading song ${progress.current} of ${progress.total}`
                  : "Reading songs..."}
              </p>
              {progress && (
                <p className="scan-wizard-muted">{progress.relative_path}</p>
              )}
            </>
          )}

          {isComplete && result && (
            <p className="scan-wizard-summary">
              {result.analyzed} analyzed, {result.skipped} skipped,{" "}
              {result.errors} with errors.
            </p>
          )}

          {isError && <p className="scan-wizard-error">{error}</p>}
        </div>

        <div className="scan-wizard-actions">
          {isComplete ? (
            <button className="primary-btn" type="button" onClick={onClose}>
              Finish
            </button>
          ) : (
            <button
              className="primary-btn"
              type="button"
              onClick={() => onAnalyze(mode)}
              disabled={isAnalyzing}
            >
              {isAnalyzing ? "Reading..." : "Start"}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}
