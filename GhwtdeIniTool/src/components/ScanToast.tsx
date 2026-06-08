import type { ScanToast as ScanToastState } from "../hooks/useModsScanner";

type ScanToastProps = {
  onDismiss: () => void;
  toast: ScanToastState;
};

export function ScanToast({ onDismiss, toast }: ScanToastProps) {
  return (
    <div className={`scan-toast scan-toast-${toast.tone}`} role="status">
      <p>{toast.message}</p>
      <button
        className="scan-toast-close"
        type="button"
        onClick={onDismiss}
        aria-label="Dismiss scan status"
      >
        &times;
      </button>
    </div>
  );
}
