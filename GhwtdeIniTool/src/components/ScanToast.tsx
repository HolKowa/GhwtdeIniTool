type ScanToastProps = {
  onDismiss: () => void;
  toast: {
    message: string;
    tone: "success" | "error";
  };
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
