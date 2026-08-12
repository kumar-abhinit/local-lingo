import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ModelInfo {
  id: string;
  filename: string;
  tier: string;
  size_mb: number;
  cached: boolean;
}

interface PermissionStatus {
  granted: boolean;
  fix_instructions: string;
}

interface DownloadProgress {
  downloaded: number;
  total: number | null;
}

interface BenchmarkResult {
  elapsed_ms: number;
  recommended_tier: "low" | "medium" | "high";
}

const STEPS = [
  "Welcome",
  "Permissions",
  "Download model",
  "Benchmark",
  "Mic test",
] as const;

export default function Onboarding() {
  const [step, setStep] = useState(0);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [permissions, setPermissions] = useState<PermissionStatus | null>(null);
  const [selectedModel, setSelectedModel] = useState("large-v3-turbo");
  const [downloadPct, setDownloadPct] = useState<number | null>(null);
  const [benchmark, setBenchmark] = useState<BenchmarkResult | null>(null);
  const [micResult, setMicResult] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<ModelInfo[]>("list_models").then(setModels).catch(console.error);
    invoke<PermissionStatus>("get_permissions")
      .then(setPermissions)
      .catch(console.error);

    const unlisten = listen<DownloadProgress>("download-progress", (e) => {
      const { downloaded, total } = e.payload;
      if (total) {
        setDownloadPct(Math.round((downloaded / total) * 100));
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function downloadModel() {
    setError("");
    try {
      await invoke<string>("download_model_cmd", { modelId: selectedModel });
      setDownloadPct(100);
    } catch (e) {
      setError(String(e));
    }
  }

  async function runBenchmark() {
    setError("");
    try {
      const result = await invoke<BenchmarkResult>("run_benchmark");
      setBenchmark(result);
      const cfg = await invoke<Record<string, unknown>>("get_config");
      await invoke("set_config", {
        config: { ...cfg, model_tier: result.recommended_tier },
      });
    } catch (e) {
      setError(String(e));
    }
  }

  async function runMicTest() {
    setError("");
    try {
      const text = await invoke<string>("mic_test_transcribe", { seconds: 3 });
      setMicResult(text || "(no speech detected)");
    } catch (e) {
      setError(String(e));
    }
  }

  async function finish() {
    await invoke("complete_onboarding");
    window.location.reload();
  }

  return (
    <div className="panel onboarding">
      <h1>Welcome to LocalLingo</h1>
      <p className="subtitle">
        100% on-device voice-to-text. No cloud. No telemetry.
      </p>

      <div className="steps">
        {STEPS.map((label, i) => (
          <span key={label} className={i === step ? "active" : ""}>
            {i + 1}. {label}
          </span>
        ))}
      </div>

      {error && <p className="error">{error}</p>}

      {step === 0 && (
        <section>
          <p>
            Press your hotkey anywhere on your system, speak, and LocalLingo types
            the transcript into whatever field has focus.
          </p>
          <ul>
            <li>Default hotkey: Ctrl+Shift+Space (Cmd+Shift+Space on macOS)</li>
            <li>Push-to-talk mode — hold to record</li>
            <li>All processing happens offline after model download</li>
          </ul>
          <button type="button" onClick={() => setStep(1)}>
            Continue
          </button>
        </section>
      )}

      {step === 1 && (
        <section>
          <h2>Permissions</h2>
          <ul>
            <li>
              <strong>Microphone</strong> — required for dictation
            </li>
            <li>
              <strong>Accessibility / input</strong> — required to type into other
              apps
            </li>
          </ul>
          {permissions && !permissions.granted && (
            <p className="warn">{permissions.fix_instructions}</p>
          )}
          <button type="button" onClick={() => setStep(2)}>
            Continue
          </button>
        </section>
      )}

      {step === 2 && (
        <section>
          <h2>Download speech model</h2>
          <select
            value={selectedModel}
            onChange={(e) => setSelectedModel(e.target.value)}
          >
            {models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.id} ({m.size_mb} MB)
                {m.cached ? " — cached" : ""}
              </option>
            ))}
          </select>
          {downloadPct !== null && (
            <div className="progress">
              <div style={{ width: `${downloadPct}%` }} />
              <span>{downloadPct}%</span>
            </div>
          )}
          <button type="button" onClick={downloadModel}>
            Download
          </button>
          <button type="button" onClick={() => setStep(3)}>
            Continue
          </button>
        </section>
      )}

      {step === 3 && (
        <section>
          <h2>Quick benchmark</h2>
          <p>Runs a short inference test to recommend the best model tier.</p>
          <button type="button" onClick={runBenchmark}>
            Run benchmark
          </button>
          {benchmark && (
            <p>
              Inference: {benchmark.elapsed_ms} ms — recommended tier:{" "}
              <strong>{benchmark.recommended_tier}</strong>
            </p>
          )}
          <button type="button" onClick={() => setStep(4)}>
            Continue
          </button>
        </section>
      )}

      {step === 4 && (
        <section>
          <h2>Mic test</h2>
          <p>Say something — we'll transcribe 3 seconds of audio.</p>
          <button type="button" onClick={runMicTest}>
            Start mic test
          </button>
          {micResult && <p className="result">{micResult}</p>}
          <button type="button" onClick={finish}>
            Finish setup
          </button>
        </section>
      )}
    </div>
  );
}
