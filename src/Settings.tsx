import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface AppConfig {
  hotkey: string;
  hotkey_mode: "push_to_talk" | "toggle";
  mic_device: string | null;
  model_tier: "low" | "medium" | "high";
  model_path: string | null;
  trailing_silence_ms: number;
  onboarding_complete: boolean;
  groq_api_key: string | null;
}

interface AudioDevice {
  name: string;
  is_default: boolean;
}

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

interface AsrStatus {
  backend: "local" | "cloud" | "unavailable";
  model_path: string | null;
  cloud_configured: boolean;
  local_model_cached: boolean;
}

function backendLabel(status: AsrStatus | null): string {
  if (!status) return "Checking…";
  switch (status.backend) {
    case "local":
      return "Local (Whisper) — fully offline";
    case "cloud":
      return "Cloud (Groq) — download a model for offline use";
    default:
      return "Unavailable — add Groq API key or download a model";
  }
}

export default function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [permissions, setPermissions] = useState<PermissionStatus | null>(null);
  const [asrStatus, setAsrStatus] = useState<AsrStatus | null>(null);
  const [status, setStatus] = useState("Ready");
  const [micTestResult, setMicTestResult] = useState("");
  const [lastTranscript, setLastTranscript] = useState("");
  const [groqKeyDraft, setGroqKeyDraft] = useState("");

  useEffect(() => {
    loadAll();
    const unsubs = [
      listen<string>("transcription", (e) => setLastTranscript(e.payload)),
      listen("mic-test-request", () => runMicTest()),
      listen<string>("hotkey-error", (e) =>
        setStatus(`Hotkey unavailable: ${e.payload}. Change the hotkey below.`),
      ),
    ];
    return () => {
      unsubs.forEach((p) => p.then((u) => u()));
    };
  }, []);

  async function loadAll() {
    try {
      const [cfg, devs, mods, perms, asr] = await Promise.all([
        invoke<AppConfig>("get_config"),
        invoke<AudioDevice[]>("list_audio_devices"),
        invoke<ModelInfo[]>("list_models"),
        invoke<PermissionStatus>("get_permissions"),
        invoke<AsrStatus>("get_asr_status"),
      ]);
      setConfig(cfg);
      setDevices(devs);
      setModels(mods);
      setPermissions(perms);
      setAsrStatus(asr);
      setGroqKeyDraft(cfg.groq_api_key ?? "");
    } catch (e) {
      setStatus(`Error loading: ${e}`);
    }
  }

  async function saveConfig(update: Partial<AppConfig>) {
    if (!config) return;
    const next = { ...config, ...update };
    try {
      await invoke("set_config", { config: next });
      setConfig(next);
      const asr = await invoke<AsrStatus>("get_asr_status");
      setAsrStatus(asr);
      setStatus("Settings saved");
    } catch (e) {
      setStatus(`Save failed: ${e}`);
      throw e;
    }
  }

  async function saveGroqKey() {
    const trimmed = groqKeyDraft.trim();
    try {
      await saveConfig({ groq_api_key: trimmed || null });
      setStatus(trimmed ? "Groq API key saved" : "Groq API key cleared");
    } catch {
      // saveConfig already set status
    }
  }

  async function downloadModel(modelId: string) {
    setStatus(`Downloading ${modelId}…`);
    try {
      const path = await invoke<string>("download_model_cmd", { modelId });
      await saveConfig({ model_path: path });
      await loadAll();
      setStatus(`Model ready: ${path}`);
    } catch (e) {
      setStatus(`Download failed: ${e}`);
    }
  }

  async function runMicTest() {
    setStatus("Recording 3 seconds…");
    setMicTestResult("");
    try {
      const text = await invoke<string>("mic_test_transcribe", { seconds: 3 });
      setMicTestResult(text || "(no speech detected)");
      setStatus("Mic test complete");
    } catch (e) {
      setStatus(`Mic test failed: ${e}`);
    }
  }

  async function testInjection() {
    try {
      await invoke("test_injection", { text: "LocalLingo injection test." });
      setStatus("Injection test sent");
    } catch (e) {
      setStatus(`Injection failed: ${e}`);
    }
  }

  if (!config) {
    return <div className="panel">Loading…</div>;
  }

  const transcriptionReady =
    asrStatus?.backend === "local" || asrStatus?.backend === "cloud";

  return (
    <div className="panel">
      <h1>LocalLingo Settings</h1>
      <p className="status">{status}</p>
      <p className={asrStatus?.backend === "cloud" ? "warn" : "ok"}>
        Transcription: {backendLabel(asrStatus)}
      </p>

      <section>
        <h2>Cloud fallback (Groq)</h2>
        <p className="hint">
          Use Groq&apos;s free Whisper API until a local model is downloaded. Audio
          is sent to Groq only while cloud mode is active.
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
        <button type="button" onClick={saveGroqKey}>
          Save API key
        </button>
        <p className="hint">
          Get a free key at{" "}
          <a href="https://console.groq.com" target="_blank" rel="noreferrer">
            console.groq.com
          </a>
          . You can also set <code>GROQ_API_KEY</code> in your environment.
        </p>
      </section>

      <section>
        <h2>Hotkey</h2>
        <label>
          Combination
          <input
            value={config.hotkey}
            onChange={(e) => saveConfig({ hotkey: e.target.value })}
          />
        </label>
        <label>
          Mode
          <select
            value={config.hotkey_mode}
            onChange={(e) =>
              saveConfig({
                hotkey_mode: e.target.value as AppConfig["hotkey_mode"],
              })
            }
          >
            <option value="push_to_talk">Push-to-talk (hold)</option>
            <option value="toggle">Toggle</option>
          </select>
        </label>
        <label>
          Trailing silence (ms)
          <input
            type="number"
            value={config.trailing_silence_ms}
            onChange={(e) =>
              saveConfig({ trailing_silence_ms: Number(e.target.value) })
            }
          />
        </label>
      </section>

      <section>
        <h2>Microphone</h2>
        <select
          value={config.mic_device ?? ""}
          onChange={(e) =>
            saveConfig({ mic_device: e.target.value || null })
          }
        >
          <option value="">System default</option>
          {devices.map((d) => (
            <option key={d.name} value={d.name}>
              {d.name}
              {d.is_default ? " (default)" : ""}
            </option>
          ))}
        </select>
        <button type="button" onClick={runMicTest} disabled={!transcriptionReady}>
          Mic test (3s)
        </button>
        {micTestResult && <p className="result">{micTestResult}</p>}
      </section>

      <section>
        <h2>Model</h2>
        <ul className="model-list">
          {models.map((m) => (
            <li key={m.id}>
              <strong>{m.id}</strong> — {m.size_mb} MB{" "}
              {m.cached ? "✓ cached" : ""}
              {!m.cached && (
                <button type="button" onClick={() => downloadModel(m.id)}>
                  Download
                </button>
              )}
            </li>
          ))}
        </ul>
        {config.model_path && (
          <p className="hint">Active: {config.model_path}</p>
        )}
      </section>

      <section>
        <h2>Permissions</h2>
        {permissions && (
          <p className={permissions.granted ? "ok" : "warn"}>
            {permissions.granted
              ? "Text injection permissions OK"
              : permissions.fix_instructions}
          </p>
        )}
        <button type="button" onClick={testInjection}>
          Test injection
        </button>
      </section>

      {lastTranscript && (
        <section>
          <h2>Last transcription</h2>
          <p className="result">{lastTranscript}</p>
        </section>
      )}
    </div>
  );
}
