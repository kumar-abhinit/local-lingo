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

export default function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [permissions, setPermissions] = useState<PermissionStatus | null>(null);
  const [status, setStatus] = useState("Ready");
  const [micTestResult, setMicTestResult] = useState("");
  const [lastTranscript, setLastTranscript] = useState("");

  useEffect(() => {
    loadAll();
    const unsubs = [
      listen<string>("transcription", (e) => setLastTranscript(e.payload)),
      listen("mic-test-request", () => runMicTest()),
    ];
    return () => {
      unsubs.forEach((p) => p.then((u) => u()));
    };
  }, []);

  async function loadAll() {
    try {
      const [cfg, devs, mods, perms] = await Promise.all([
        invoke<AppConfig>("get_config"),
        invoke<AudioDevice[]>("list_audio_devices"),
        invoke<ModelInfo[]>("list_models"),
        invoke<PermissionStatus>("get_permissions"),
      ]);
      setConfig(cfg);
      setDevices(devs);
      setModels(mods);
      setPermissions(perms);
    } catch (e) {
      setStatus(`Error loading: ${e}`);
    }
  }

  async function saveConfig(update: Partial<AppConfig>) {
    if (!config) return;
    const next = { ...config, ...update };
    await invoke("set_config", { config: next });
    setConfig(next);
    setStatus("Settings saved");
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

  return (
    <div className="panel">
      <h1>LocalLingo Settings</h1>
      <p className="status">{status}</p>

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
        <button type="button" onClick={runMicTest}>
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
