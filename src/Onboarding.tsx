import { useCallback, useEffect, useState } from "react";
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
  const [downloading, setDownloading] = useState(false);
  const [downloadPct, setDownloadPct] = useState<number | null>(null);
  const [benchmark, setBenchmark] = useState<BenchmarkResult | null>(null);
  const [micResult, setMicResult] = useState("");
  const [error, setError] = useState("");

  const refreshModels = useCallback(async () => {
    const list = await invoke<ModelInfo[]>("list_models");
    setModels(list);
    return list;
  }, []);

  useEffect(() => {
    refreshModels().catch(console.error);
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
  }, [refreshModels]);

  const selectedModelInfo = models.find((m) => m.id === selectedModel);
  const anyModelCached = models.some((m) => m.cached);

  async function ensureModelConfigured(): Promise<boolean> {
    const cached =
      models.find((m) => m.id === selectedModel && m.cached) ??
      models.find((m) => m.cached);
    if (!cached) {
      return false;
    }
    const path = await invoke<string>("download_model_cmd", { modelId: cached.id });
    const cfg = await invoke<Record<string, unknown>>("get_config");
    await invoke("set_config", {
      config: {
        ...cfg,
        model_path: path,
        model_tier: cached.tier,
      },
    });
    if (cached.id !== selectedModel) {
      setSelectedModel(cached.id);
    }
    return true;
  }

  function goBack() {
    setError("");
    setStep((s) => Math.max(0, s - 1));
  }

  function goNext(next: number) {
    setError("");
    setStep(next);
  }

  async function downloadModel() {
    setError("");
    setDownloading(true);
    setDownloadPct(0);
    try {
      const path = await invoke<string>("download_model_cmd", { modelId: selectedModel });
      const cfg = await invoke<Record<string, unknown>>("get_config");
      const info = models.find((m) => m.id === selectedModel);
      await invoke("set_config", {
        config: {
          ...cfg,
          model_path: path,
          ...(info ? { model_tier: info.tier } : {}),
        },
      });
      setDownloadPct(100);
      await refreshModels();
    } catch (e) {
      setError(String(e));
      setDownloadPct(null);
    } finally {
      setDownloading(false);
    }
  }

  async function runBenchmark() {
    setError("");
    try {
      if (!(await ensureModelConfigured())) {
        setError("Download a speech model before running the benchmark.");
        return;
      }
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
      if (!(await ensureModelConfigured())) {
        setError("Download a speech model before running the mic test.");
        return;
      }
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

  function modelDownloadBlock(hint?: string) {
    return (
      <div className="download-block">
        {hint && <p className="hint">{hint}</p>}
        <select
          value={selectedModel}
          onChange={(e) => setSelectedModel(e.target.value)}
          disabled={downloading}
        >
          {models.length === 0 ? (
            <option value={selectedModel}>Loading models…</option>
          ) : (
            models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.id} ({m.size_mb} MB)
                {m.cached ? " — cached" : ""}
              </option>
            ))
          )}
        </select>
        {downloadPct !== null && (
          <div className="progress">
            <div style={{ width: `${downloadPct}%` }} />
            <span>{downloadPct}%</span>
          </div>
        )}
        <button type="button" onClick={downloadModel} disabled={downloading || !!selectedModelInfo?.cached}>
          {downloading
            ? "Downloading…"
            : selectedModelInfo?.cached
              ? "Downloaded"
              : "Download model"}
        </button>
        {anyModelCached && <p className="ok">Model ready — you can continue.</p>}
      </div>
    );
  }

  return (
    <div className="panel onboarding">
      <h1>Welcome to LocalLingo</h1>
      <p className="subtitle">
        100% on-device voice-to-text. No cloud. No telemetry.
      </p>

      <div className="steps">
        {STEPS.map((label, i) => (
          <button
            key={label}
            type="button"
            className={`step-pill${i === step ? " active" : ""}${i < step ? " done" : ""}`}
            onClick={() => i < step && goNext(i)}
            disabled={i > step}
            title={i < step ? `Go back to ${label}` : undefined}
          >
            {i + 1}. {label}
          </button>
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
          <div className="actions">
            <button type="button" onClick={() => goNext(1)}>
              Continue
            </button>
          </div>
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
          <div className="actions">
            <button type="button" className="secondary" onClick={goBack}>
              Back
            </button>
            <button type="button" onClick={() => goNext(2)}>
              Continue
            </button>
          </div>
        </section>
      )}

      {step === 2 && (
        <section>
          <h2>Download speech model</h2>
          <p className="hint">
            Choose a model and download it before continuing. This is a one-time
            download (~200–550 MB depending on model).
          </p>
          {modelDownloadBlock()}
          <div className="actions">
            <button type="button" className="secondary" onClick={goBack}>
              Back
            </button>
            <button
              type="button"
              onClick={() => goNext(3)}
              disabled={!anyModelCached}
              title={anyModelCached ? undefined : "Download a model first"}
            >
              Continue
            </button>
          </div>
        </section>
      )}

      {step === 3 && (
        <section>
          <h2>Quick benchmark</h2>
          <p>Runs a short inference test to recommend the best model tier.</p>
          {!anyModelCached && (
            <>
              <p className="warn">
                No speech model is installed yet. Download one below or go back to
                the download step.
              </p>
              {modelDownloadBlock("Download required before benchmarking:")}
            </>
          )}
          <button type="button" onClick={runBenchmark} disabled={!anyModelCached}>
            Run benchmark
          </button>
          {benchmark && (
            <p>
              Inference: {benchmark.elapsed_ms} ms — recommended tier:{" "}
              <strong>{benchmark.recommended_tier}</strong>
            </p>
          )}
          <div className="actions">
            <button type="button" className="secondary" onClick={goBack}>
              Back
            </button>
            <button type="button" onClick={() => goNext(4)} disabled={!anyModelCached}>
              Continue
            </button>
          </div>
        </section>
      )}

      {step === 4 && (
        <section>
          <h2>Mic test</h2>
          <p>Say something — we&apos;ll transcribe 3 seconds of audio.</p>
          {!anyModelCached && (
            <>
              <p className="warn">
                No speech model is installed yet. Download one below or go back.
              </p>
              {modelDownloadBlock("Download required before mic test:")}
            </>
          )}
          <button type="button" onClick={runMicTest} disabled={!anyModelCached}>
            Start mic test
          </button>
          {micResult && <p className="result">{micResult}</p>}
          <div className="actions">
            <button type="button" className="secondary" onClick={goBack}>
              Back
            </button>
            <button type="button" onClick={finish} disabled={!anyModelCached}>
              Finish setup
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
