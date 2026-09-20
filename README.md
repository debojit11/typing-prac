<p align="center">
  <img src="src-tauri/icon.svg" width="112" alt="TypingPrac logo">
</p>

<h1 align="center">TypingPrac</h1>

<p align="center">Deliberate touch-typing key-pair practice for Windows.</p>

TypingPrac complements guided typing courses with focused repetition. Choose the key pairs you have learned, practise them in structured drills, and build accuracy and consistency before speed. The app runs entirely offline and has no telemetry or network features.

## Features

- Standalone drills for a single key pair
- Mixed drills that use only the selected pairs
- Structured exercises for repetition, alternation, transitions, and longer sequences
- Adaptive practice that gives extra attention to difficult keys and transitions
- Short, medium, long, and endless sessions
- Live accuracy, WPM, raw WPM, error, and timing metrics
- Session results and per-account progress
- Dark and light themes, adjustable text size, and an optional keyboard guide

## Getting started

### Requirements

- Windows 11 and Microsoft Edge WebView2 (normally preinstalled)
- [Rust](https://www.rust-lang.org/tools/install) stable with the MSVC toolchain
- Microsoft C++ Build Tools
- [Node.js](https://nodejs.org/) 20 or newer

### Run locally

```powershell
git clone --branch dev https://github.com/debojit11/typing-prac.git
cd typing-prac
npm install
npm run dev
```

## Commands

| Command | Purpose |
| --- | --- |
| `npm run dev` | Start the app in development mode |
| `npm test` | Run the Rust test suite |
| `npm run build` | Create an optimized Windows executable and NSIS installer |

Production output is written to `src-tauri\target\release`. The installer is placed in `src-tauri\target\release\bundle\nsis`.

## Performance

Measured from an optimized Windows 11 release build. RAM includes TypingPrac and its WebView2 child processes; results vary by machine and WebView2 version.

| Metric | Measured result |
| --- | ---: |
| Release executable | 3.02 MiB |
| NSIS installer | 1.08 MiB |
| Shipped frontend | 23.82 KiB |
| Idle private memory | 160.98 MiB |
| Active-practice private memory | 168.24 MiB |
| Idle CPU over 5 seconds | 0.000% |
| CPU at approximately 16 keystrokes/second | 0.417% total CPU |

Typing feedback is immediate. Statistics accumulate in memory during practice and are persisted once when a session finishes, never on every keystroke.

## Technology

- [Tauri 2](https://tauri.app/)
- Rust for exercise generation, statistics, accounts, and persistence
- Vanilla TypeScript, HTML, and CSS for the interface
- Argon2id password hashing

## Contributing

Issues and pull requests are welcome. Before submitting a change, run:

```powershell
npm test
npm run build
```

Please report bugs through [GitHub Issues](https://github.com/debojit11/typing-prac/issues).

## License

No license has been added yet.
