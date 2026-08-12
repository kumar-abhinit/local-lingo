#!/usr/bin/env bash
# LocalLingo WER benchmark script
# Usage: ./scripts/benchmark_wer.sh [model_path] [wav_dir]
#
# Requires: python3 with jiwer (pip install jiwer), local-lingo CLI or whisper binary
#
# For MVP, compares reference transcripts in wav_dir/*.txt against
# transcripts produced by debug WAV recording + manual ASR run.

set -euo pipefail

MODEL_PATH="${1:-${HOME}/.local/share/local-lingo/models/ggml-small.en-q5_0.bin}"
WAV_DIR="${2:-./test-data/dev-jargon}"

if [[ ! -d "$WAV_DIR" ]]; then
  echo "Creating sample test-data layout at $WAV_DIR"
  mkdir -p "$WAV_DIR"
  cat > "$WAV_DIR/README.md" <<'EOF'
# Dev jargon test set

Place paired files here:
- sentence-01.wav + sentence-01.txt (reference transcript)
- sentence-02.wav + sentence-02.txt
- ...

Recommended 20 sentences covering: async await, kubernetes, npm install,
pull request, API, CI/CD, and punctuation cues (comma, period, new line).
EOF
  echo "Add WAV + reference .txt files, then re-run."
  exit 0
fi

if ! command -v python3 &>/dev/null; then
  echo "python3 required for WER computation"
  exit 1
fi

python3 - <<'PY'
import glob, os, sys, subprocess, json

try:
    import jiwer
except ImportError:
    print("Install jiwer: pip install jiwer")
    sys.exit(1)

wav_dir = os.environ.get("WAV_DIR", "./test-data/dev-jargon")
model = os.environ.get("MODEL_PATH", "")

pairs = []
for wav in sorted(glob.glob(os.path.join(wav_dir, "*.wav"))):
    ref = wav.replace(".wav", ".txt")
    if os.path.exists(ref):
        with open(ref) as f:
            pairs.append((wav, f.read().strip()))

if not pairs:
    print(f"No wav/txt pairs found in {wav_dir}")
    sys.exit(1)

print(f"Found {len(pairs)} test pairs")
print("Note: wire this script to `local-lingo transcribe` CLI when available.")
print("For now, report reference transcripts only:")

for wav, ref in pairs:
    print(f"  {os.path.basename(wav)}: {ref[:60]}...")

# Placeholder WER — replace with actual ASR output comparison
hypotheses = [ref for _, ref in pairs]  # perfect baseline placeholder
wer = jiwer.wer([r for _, r in pairs], hypotheses)
print(f"\nBaseline WER (identity): {wer:.2%}")
print("Replace hypotheses with ASR output to measure real WER.")
PY
