# 🚀 Introducing cowcow v0.1 — Capture Our Words / Capture Our World

Hey everyone! 👋

We're excited to open a dedicated discussion thread for **cowcow**, our new offline-first speech data collection toolkit focused on endangered and marginalized languages.

## 🌱 What is cowcow?

| Layer | Tech | Purpose |
|-------|------|---------|
| `cowcow_core` | Rust 🔧 + FFI | Ultra-light audio DSP & QC (SNR / clipping / VAD) that compiles to desktop, Android, iOS. |
| `cowcow_cli` | Rust 🦀 | Cross-platform command-line tool for recording (`cowcow record --lang sw`), local QC, and resumable uploads. |
| `cowcow_flutter` | Flutter 🦄 | Offline-first mobile app with prompt carousel, token-rewards wallet, and sync toggle (Wi-Fi / any / manual). |
| `server` | FastAPI 🐍 + gRPC | Reference backend that lands files in S3-compatible bucket (e.g., Cloudflare R2) and tracks rewards. |

### Core goals:

✅ **Zero-barrier capture** – works on a $50 Android phone or a Raspberry Pi.

✅ **No-connectivity OK** – queue in SQLite, sync when bandwidth returns.

✅ **Immediate QA feedback** – SNR meter + clipping warning while the speaker records.

✅ **Contributor incentives** – airtime / M-Pesa payouts or "data tokens" per validated clip.

✅ **Simple APIs** – gRPC streaming + REST fallback; easy to plug into any data-lake.

## ⚡ Why does this matter?

> *"93% of the world's languages don't have a single open-source ASR model.  
> Yet every day, words disappear forever when elders pass."*

Platforms like Label Studio and Audino expect constant broadband and often choke on low-end hardware. **cowcow** is designed for 2G villages, field linguists, and roaming gig workers who can't rely on LTE.

## 🛠️ Current Status (v0.1 alpha)

✅ Rust core `analyze_wav()` — returns SNR, clipping %, VAD ratio.

✅ CLI: `record`, `upload`, `export`, `stats`, `doctor`.

✅ Mobile UI skeleton (Android build tested on budget devices).

✅ FastAPI demo server with JWT auth & gRPC upload stream.

🚧 Whisper-tiny on-device ASR preview (optional feature flag).

🚧 Token-reward smart-contract stub.

🚧 Docs site & API reference.

## 📅 Roadmap (next 6 weeks)

| Week | Milestone |
|------|-----------|
| 1-2 | Finish Flutter waveform visual + live QC overlay. |
| 2-3 | Add face/plate blurring filter (TensorRT) for video mode. |
| 3-4 | Release Docker compose for one-click backend deploy. |
| 4-5 | "Contributor dashboard" React plug-in for any web portal. |
| 6 | Tag v0.2, publish blog + call-for-datasets (3 pilot languages). |

## 💬 How you can help

🧪 **Test the CLI** on macOS, Linux, Windows — report audio-device edge-cases.

🌍 **Translate UI strings** (JSON) into your language (especially Swahili, Yoruba, Zulu, Amharic).

💰 **Design reward economics** — ideas for fair token or airtime payouts? Chime in!

🔧 **Build integrations** — Got a FastAPI or Django backend? Show a snippet that plugs cowcow uploads into your pipeline.

📝 **Suggest prompts** / story scripts for the open prompt-library (`/prompts/` directory).

## 🏁 Quick Start Snap-Shot

```bash
# 1. Install CLI
cargo build --release

# 2. Record a Swahili clip (auto-stops after 5s silence)
./target/release/cowcow_cli record --lang sw --prompt "Habari za asubuhi"

# 3. Upload when Wi-Fi returns
./target/release/cowcow_cli upload --endpoint https://api.cowcow.local
```

**Pro-tip**: `cowcow_cli doctor` prints audio-device diagnostics and mic gain suggestions.

## 📚 Resources

**Repo**: https://github.com/thabhelo/cowcow

**Docs preview**: `/docs/index.md` (will move to Docusaurus site soon)

**Architecture diagram**: `/docs/architecture.md`

**Issue board**: https://github.com/thabhelo/cowcow/issues

## 🤝 Join the conversation

Drop questions, bug reports, or wild feature ideas below.

If you'd like to become a core contributor, comment **"✋ I'm in"** and tell us which part (Rust, Flutter, DevOps, UX) fires you up.

Let's capture our words and keep our cultures alive — one clip at a time. 🌍🎙️

---

**— The CowCow team** 