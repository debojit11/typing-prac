<img src="src-tauri/icon.svg" width="88" alt="TypingPrac logo">

# TypingPrac

A lightweight desktop app for deliberate touch-typing practice.

<!-- Add the main practice-screen screenshot here when one is available. Suggested path: docs/typingprac-practice.png -->

## Why this exists

Typing courses are useful for introducing keys, but they often move on before a learner has built reliable muscle memory. TypingPrac provides focused repetition for the keys already learned, complementing a typing curriculum rather than replacing one.

## Features

- Standalone practice for one key pair
- Custom mixed practice across several selected pairs
- Structured progression from repetition to difficult transitions
- Directed transition coverage and balanced key exposure
- Adaptive reinforcement for weak keys and transitions
- Short, Medium, Long, and Endless sessions
- Accuracy-first live feedback and session results
- Pause, resume, restart, Backspace, and Stop-on-Error controls
- Optional target-key and finger guide
- Local progress statistics and separate local profiles
- Fully offline operation with no telemetry or cloud services
- Small Tauri and Rust application with a framework-free frontend

## Download

[Download the latest Windows installer](https://github.com/debojit11/typing-prac/releases/latest). Windows may show a SmartScreen warning because the installer is not code-signed.

## Getting started

### Requirements

- Windows 11 with Microsoft Edge WebView2
- Rust and Cargo using the stable MSVC toolchain
- Microsoft C++ Build Tools with **Desktop development with C++**
- Node.js 20 or newer with npm

### Run locally

```powershell
git clone --branch dev https://github.com/debojit11/typing-prac.git
cd typing-prac
npm install
npm run dev
```

### Commands

| Task | Command |
| --- | --- |
| Type-check and bundle the frontend | `npm run frontend` |
| Run frontend and Rust tests | `npm test` |
| Inspect generated practice samples | `npm run inspect` |
| Create an optimized executable and installer | `npm run build` |

The release executable is written to `src-tauri/target/release/typing-prac.exe`. The NSIS installer is written under `src-tauri/target/release/bundle/nsis/`.

## Practice modes

### Standalone practice

Standalone practice trains one pair deeply. A session moves through repeated keys, simple and uneven alternation, mirrored patterns, recovery after repeated keys, short transitions, and longer sequences.

### Mixed practice

Mixed practice combines every selected pair and deliberately covers transitions between pairs. It does not concatenate standalone drills. The generator balances the selected keys while building same-hand, cross-hand, pair-to-pair, and directional transition exercises.

### Adaptive practice

Adaptive practice gives bounded extra weight to keys and transitions associated with errors or slow response times. Weak material receives more exposure without taking over the session, and its weight falls as performance improves.

## Default key progression

| Row | Key pairs, in order |
| --- | --- |
| Home | `F / J`, `D / K`, `S / L`, `A / ;`, `G / H` |
| Top | `R / U`, `E / I`, `W / O`, `Q / Y`, `T / P` |
| Bottom | `V / M`, `C / ,`, `X / .`, `Z / /`, `B / N` |

## Performance

Low idle overhead and immediate input feedback are explicit design constraints. Recent measurements from an optimized Windows 11 build are:

| Measurement | Approximate result |
| --- | ---: |
| Release executable | 3.03 MiB |
| NSIS installer | 1.08 MiB |
| Minified frontend JavaScript | 15.9 KiB |
| Idle root-process CPU | 0% |
| Idle total private memory | 161 MiB |
| Active-practice total private memory | 168 MiB |

Memory figures include the WebView2 process group, which contributes most of the total and varies with the installed WebView2 runtime. These measurements are reference values, not hardware-independent guarantees.

## Tech stack

| Layer | Technology |
| --- | --- |
| Desktop shell | Tauri 2 |
| Core logic | Rust |
| Frontend | Vanilla TypeScript, HTML, and CSS |
| Bundling | esbuild |
| Primary platform | Windows 11 |

The frontend deliberately avoids React and other component frameworks. Typing feedback is handled locally in the webview without per-keystroke backend calls or disk writes.

## Development

| Area | Path |
| --- | --- |
| Practice generator and generator tests | `src-tauri/src/generator.rs` |
| Rust commands and application setup | `src-tauri/src/lib.rs` |
| Data models and statistics | `src-tauri/src/model.rs` |
| Local authentication and persistence | `src-tauri/src/auth.rs`, `src-tauri/src/storage.rs` |
| Frontend application and input handling | `src/main.ts` |
| Session state and UX logic | `src/session.ts` |
| Frontend UX tests | `src/session.test.mjs` |
| Markup and styles | `frontend/index.html`, `frontend/styles.css` |
| Tauri configuration | `src-tauri/tauri.conf.json` |

### Practice generator design

Exercise generation is structured rather than uniformly random. Standalone sessions use pair-specific repetition, alternation, mirrored patterns, recovery patterns, and staged difficulty. Mixed sessions use a separate strategy that schedules directional transitions across selected pairs, balances exposure, and retains a small recent-history window to limit accidental exact repeats.

Difficulty increases by section. Most deliberate work uses short chunks, harder sections introduce medium-length combinations, and the densest sequences are reserved mainly for endurance or longer sessions.

### Adaptive weighting

The local profile aggregates errors, response times, and practice counts for keys and transitions. The generator converts those signals into capped weights, so a weak transition is reinforced but broad coverage remains intact. Successful practice reduces its relative weight over time. This is deterministic weighting over local statistics, not machine learning.

## Local data and privacy

TypingPrac has no online account, telemetry, analytics, cloud sync, or network requirement. It does use local username/password profiles so multiple learners can keep separate progress on one computer. Passwords are stored as Argon2id hashes, not plaintext.

Tauri resolves the application data directory for the current operating system. On Windows, profiles, settings, and aggregated practice statistics are stored in:

```text
%APPDATA%\com.typingprac.desktop\accounts.json
```

Persistence is batched around meaningful events such as session completion or settings changes. Individual keystrokes are not written to disk.

## Testing

`npm test` runs the frontend session tests followed by the Rust test suite. The Rust tests cover generator constraints, balance, transition coverage, adaptive weighting, statistics, authentication, and persistence-related models. The frontend tests cover session completion, pause timing, error handling, Backspace behavior, state transitions, and punctuation input.

`npm run frontend` performs the TypeScript type check and creates the production frontend bundle. `npm run inspect` runs the ignored developer inspection test and prints standalone, mixed, and adaptive sample exercises for manual quality review.

## Design principles

- Accuracy before speed
- Deliberate practice instead of random noise
- Immediate, synchronous typing feedback
- Offline operation and local ownership of data
- Minimal dependencies and resource use
- A simple interface that keeps attention on the exercise

## Possible future work

- Support additional keyboard layouts
- Export and import local progress
- Add clearer long-term progress trends

These are possible directions rather than committed features.

## Contributing

Issues and pull requests are welcome. Changes should preserve offline operation, keep dependencies justified, avoid per-keystroke IPC or disk writes, and include focused tests when generator or session behavior changes.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
