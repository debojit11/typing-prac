# TypingPrac

TypingPrac is a small, offline Windows desktop app for deliberate key-pair repetition. Accuracy is deliberately more prominent than speed. It uses Tauri, a Rust practice/persistence core, and a framework-free TypeScript UI.

## Requirements

- Windows 11 with the Microsoft Edge WebView2 runtime (normally preinstalled)
- Rust stable with the MSVC target and Microsoft C++ Build Tools
- Node.js 20 or newer

## Run and build

```powershell
npm install
npm run dev
```

Run core tests with `npm test`. Build the optimized app and NSIS installer with `npm run build`. Outputs are written below `src-tauri\target\release`.

## Generator

Standalone mode moves through repetition, alternation, uneven runs, difficult transitions, and longer structured sequences using one pair. Mixed mode is separate: it schedules balanced pair exposure, left/right alternation, same-hand runs, pair-to-pair transitions, cross-pair transitions, and longer sequences. A small seeded xorshift generator makes tests reproducible without adding a random-number dependency.

Adaptive mode raises a problem transition's selection weight up to 3×. It remains mixed with normal material, and aggregated timing/error evidence is used rather than retaining raw keystroke logs.

## Local data

Accounts, settings, and per-account aggregated statistics are stored locally in `%APPDATA%\com.typingprac.desktop\accounts.json`. Passwords are never stored directly: each password is protected with a unique salt and Argon2id hash. Only the 20 latest session summaries per account are retained; old keystrokes are never stored individually. Key and transition counts/timings remain as small aggregates.

Accounts are local profiles, not cloud identities. They keep progress separate and deter casual access inside the app, but cannot protect data from someone who already controls your Windows account. There is deliberately no password recovery mechanism because no email or server exists.

## Add a key pair

Add its two-character lowercase value in three places: `PAIRS` in `src-tauri/src/generator.rs`, the appropriate group in `src/main.ts`, and the desired position in both lists. The ordering in those two arrays is the learning progression shown in the UI and accepted by the generator.
