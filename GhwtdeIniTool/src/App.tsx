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
    } catch {
      updateRef.current = null;
      setUpdateStatus("none");
    }
  }

  async function startDownload(update: Update) {
    setUpdateStatus("downloading");
    setDownloadProgress(0);
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
      setUpdateStatus("error");
      setErrorMessage(String(err));
    }
  }

  function handleSkip() {
    updateRef.current = null;
    setUpdateStatus("none");
  }

  const shouldShowUpdatePopup =
    updateStatus === "available" ||
    updateStatus === "downloading" ||
    updateStatus === "ready" ||
    updateStatus === "error";

  return (
    <main className="container">
      <h1>GhwtdeIniTool</h1>
      <p>Welcome to GhwtdeIniTool.</p>

      {shouldShowUpdatePopup && (
        <div className="update-popup-backdrop" role="presentation">
          <div
            className={`update-popup ${updateStatus}`}
            role="dialog"
            aria-modal="true"
            aria-labelledby="update-popup-title"
          >
            {updateStatus === "available" && (
              <>
                <h2 id="update-popup-title">An update is available</h2>
                <p>A newer version of GhwtdeIniTool is ready to download.</p>
                <div className="update-buttons">
                  <button
                    className="update-now-btn"
                    onClick={() => {
                      const update = updateRef.current;
                      if (update) {
                        startDownload(update);
                      }
                    }}
                  >
                    Update
                  </button>
                  <button className="skip-btn" onClick={handleSkip}>
                    Skip
                  </button>
                </div>
              </>
            )}

            {updateStatus === "downloading" && (
              <>
                <h2 id="update-popup-title">Downloading update</h2>
                <p>{downloadProgress}% complete</p>
                <div className="progress-bar">
                  <div
                    className="progress-fill"
                    style={{ width: `${downloadProgress}%` }}
                  />
                </div>
              </>
            )}

            {updateStatus === "ready" && (
              <>
                <h2 id="update-popup-title">Update downloaded</h2>
                <p>Restart GhwtdeIniTool to install the update.</p>
                <button className="install-btn" onClick={handleInstall}>
                  Restart & Install
                </button>
              </>
            )}

            {updateStatus === "error" && (
              <>
                <h2 id="update-popup-title">Update failed</h2>
                <p>{errorMessage}</p>
                <button className="skip-btn" onClick={handleSkip}>
                  Close
                </button>
              </>
            )}
          </div>
        </div>
      )}
    </main>
  );
}

export default App;
