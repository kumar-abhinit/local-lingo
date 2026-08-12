import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Settings from "./Settings";
import Onboarding from "./Onboarding";
import "./App.css";

function App() {
  const [onboardingComplete, setOnboardingComplete] = useState<boolean | null>(
    null,
  );

  useEffect(() => {
    invoke<{ onboarding_complete: boolean }>("get_config")
      .then((cfg) => setOnboardingComplete(cfg.onboarding_complete))
      .catch(() => setOnboardingComplete(false));
  }, []);

  if (onboardingComplete === null) {
    return (
      <main className="container">
        <p>Loading LocalLingo…</p>
      </main>
    );
  }

  return (
    <main className="container">
      {onboardingComplete ? <Settings /> : <Onboarding />}
    </main>
  );
}

export default App;
