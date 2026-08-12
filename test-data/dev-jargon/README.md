# Dev jargon test set

Add paired WAV + reference transcript files:

- `sentence-01.wav` + `sentence-01.txt`
- `sentence-02.wav` + `sentence-02.txt`
- ...

Recommended 20 sentences covering: async await, kubernetes, npm install,
pull request, API, CI/CD, and punctuation cues (comma, period, new line).

Run benchmark:

```bash
./scripts/benchmark_wer.sh
```
