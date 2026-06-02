import { useState, useEffect, useRef } from "react";
import { check } from "@tauri-apps/plugin-updater";
import type { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import "./App.css";

function App() {
  const [updateStatus, setUpdateStatus] = useState<
    | "checking"
    | "available"
    | "downloading"
    | "ready"
    | "skipped"
    | "none"
    | "error"
  >("checking");
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [errorMessage, setErrorMessage] = useState("");
  const updateRef = useRef<Update | null>(null);

  useEffect(() => {
    checkForUpdates();
  }, []);

  async function checkForUpdates() {
    try {
      const update = await check();
      if (!update) {
        setUpdateStatus("none");
        return;
      }

      updateRef.current = update;
      setUpdateStatus("available");
    } catch (err) {
      setUpdateStatus("error");
      setErrorMessage(String(err));
    }
  }

  async function startDownload(update: Update) {
    setUpdateStatus("downloading");
    let downloaded = 0;
    let contentLength = 0;

    try {
      await update.download(
        (event) => {
          switch (event.event) {
            case "Started":
              contentLength = event.data.contentLength ?? 0;
              break;
            case "Progress":
              downloaded += event.data.chunkLength;
              if (contentLength > 0) {
                setDownloadProgress(
                  Math.round((downloaded / contentLength) * 100)
                );
              }
              break;
            case "Finished":
              setDownloadProgress(100);
              break;
          }
        }
      );

      setUpdateStatus("ready");
    } catch (err) {
      setUpdateStatus("error");
      setErrorMessage(String(err));
    }
  }

  async function handleInstall() {
    try {
      const update = updateRef.current;
      if (update) {
        await update.install();
        await relaunch();
      }
    } catch (err) {
      setErrorMessage(String(err));
    }
  }

  function handleSkip() {
    setUpdateStatus("skipped");
  }

  return (
    <main className="container">
      <h1>GhwtdeIniTool</h1>
      <p>Welcome to GhwtdeIniTool.</p>

      <div className="update-section">
        {updateStatus === "checking" && (
          <div className="update-card checking">
            <span className="update-spinner" />
            <span>Checking for updates…</span>
          </div>
        )}

        {updateStatus === "none" && (
          <div className="update-card up-to-date">
            <span>✓ You're on the latest version</span>
          </div>
        )}

        {updateStatus === "available" && (
          <div className="update-card available">
            <span>⬇ An update is available</span>
            <div className="update-buttons">
              <button
                className="update-now-btn"
                onClick={() => startDownload(updateRef.current!)}
              >
                Update Now
              </button>
              <button className="skip-btn" onClick={handleSkip}>
                Skip
              </button>
            </div>
          </div>
        )}

        {updateStatus === "skipped" && (
          <div className="update-card skipped">
            <span>Update skipped — check again on next launch</span>
          </div>
        )}

        {updateStatus === "downloading" && (
          <div className="update-card downloading">
            <span>Downloading update… {downloadProgress}%</span>
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${downloadProgress}%` }}
              />
            </div>
          </div>
        )}

        {updateStatus === "ready" && (
          <div className="update-card ready">
            <span>✓ Update downloaded!</span>
            <button className="install-btn" onClick={handleInstall}>
              Restart & Install
            </button>
          </div>
        )}

        {updateStatus === "error" && (
          <div className="update-card error">
            <span>✗ Update check failed: {errorMessage}</span>
          </div>
        )}
      </div>
    </main>
  );
}

export default App;
