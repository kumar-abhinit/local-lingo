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
  const [groqKeyDraft, setGroqKeyDraft] = useState("");
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
    invoke<{ groq_api_key: string | null }>("get_config")
      .then((cfg) => setGroqKeyDraft(cfg.groq_api_key ?? ""))
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
  const groqConfigured = groqKeyDraft.trim().length > 0;
  const canTranscribe = anyModelCached || groqConfigured;

  async function saveGroqKey() {
    setError("");
    const trimmed = groqKeyDraft.trim();
    const cfg = await invoke<Record<string, unknown>>("get_config");
    await invoke("set_config", {
      config: { ...cfg, groq_api_key: trimmed || null },
    });
  }

  async function ensureModelConfigured(): Promise<boolean> {
    const cached =
      models.find((m) => m.id === selectedModel && m.cached) ??
      models.find((m) => m.cached);
    if (!cached) {
      return groqConfigured;
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
      if (!anyModelCached) {
        setError("Benchmark requires a local model — download one or skip to Mic test.");
        return;
      }
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
      await saveGroqKey();
      if (!(await ensureModelConfigured()) && !groqConfigured) {
        setError("Download a model or add a Groq API key before running the mic test.");
        return;
      }
      const text = await invoke<string>("mic_test_transcribe", { seconds: 3 });
      setMicResult(text || "(no speech detected)");
    } catch (e) {
      setError(String(e));
    }
  }

  async function finish() {
    await saveGroqKey();
    await invoke("complete_onboarding");
    window.location.reload();
  }

  function groqKeyBlock() {
    return (
      <div className="download-block">
        <p className="hint">
          Optional: use Groq&apos;s free Whisper API until you download a local model.
          Audio is sent to Groq only while no local model is installed.
        </p>
        <label>
          Groq API key
          <input
            type="password"
            value={groqKeyDraft}
            placeholder="gsk_…"
            onChange={(e) => setGroqKeyDraft(e.target.value)}
          />
        </label>
        <p className="hint">
          Free key at{" "}
          <a href="https://console.groq.com" target="_blank" rel="noreferrer">
            console.groq.com
          </a>
        </p>
        {groqConfigured && !anyModelCached && (
          <p className="ok">Cloud transcription ready — you can skip download for now.</p>
        )}
      </div>
    );
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
        {anyModelCached && <p className="ok">Local model ready.</p>}
      </div>
    );
  }

  return (
    <div className="panel onboarding">
      <h1>Welcome to LocalLingo</h1>
      <p className="subtitle">
        Voice-to-text on your desktop. Offline after model download, or start instantly with Groq.
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
            <li>Start with Groq cloud transcription, or download a model for offline use</li>
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
            Download a local model for fully offline use (~200–550 MB), or add a
            Groq API key to start immediately.
          </p>
          {groqKeyBlock()}
          {modelDownloadBlock()}
          <div className="actions">
            <button type="button" className="secondary" onClick={goBack}>
              Back
            </button>
            <button
              type="button"
              onClick={async () => {
                await saveGroqKey();
                goNext(3);
              }}
              disabled={!canTranscribe}
              title={canTranscribe ? undefined : "Download a model or add a Groq API key"}
            >
              Continue
            </button>
          </div>
        </section>
      )}

      {step === 3 && (
        <section>
          <h2>Quick benchmark</h2>
          <p>Runs a short local inference test to recommend the best model tier.</p>
          {!anyModelCached && (
            <>
              <p className="warn">
                {groqConfigured
                  ? "Benchmark needs a local model. You can skip to Mic test and use Groq cloud transcription."
                  : "No local model installed. Download one below or go back to add a Groq API key."}
              </p>
              {modelDownloadBlock("Download a local model for benchmarking:")}
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
            <button type="button" onClick={() => goNext(4)} disabled={!canTranscribe}>
              Continue
            </button>
          </div>
        </section>
      )}

      {step === 4 && (
        <section>
          <h2>Mic test</h2>
          <p>Say something — we&apos;ll transcribe 3 seconds of audio.</p>
          {!canTranscribe && (
            <>
              <p className="warn">
                Add a Groq API key or download a model to run the mic test.
              </p>
              {groqKeyBlock()}
              {modelDownloadBlock()}
            </>
          )}
          <button type="button" onClick={runMicTest} disabled={!canTranscribe}>
            Start mic test
          </button>
          {micResult && <p className="result">{micResult}</p>}
          <div className="actions">
            <button type="button" className="secondary" onClick={goBack}>
              Back
            </button>
            <button type="button" onClick={finish} disabled={!canTranscribe}>
              Finish setup
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
