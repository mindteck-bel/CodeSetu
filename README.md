# CodeSetu

Local desktop app that bridges **intranet** and **internet** copies of the same codebase.

You copy one side onto the other PC (USB), pick both folders, review what changed, and apply only the shared changes. CodeSetu never pushes and never needs a server.

## Why

Folder diff tools forget your decisions. Sync tools overwrite local work. CodeSetu remembers:

- intranet-only and internet-only paths (`codesetu.toml`)
- what you already accepted last time (local snapshot)
- safe auto-choices, so you only sit with **Needs you**

## Quick start

On Windows you need [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload (MSVC linker). Git’s `usr/bin/link.exe` must not shadow it.

```bash
npm install
npm run tauri dev
```

Build a Windows installer:

```bash
npm run tauri build
```

GitHub Actions builds the installer on every push to `main` — download the artifact from the workflow run.

## Project rules

Copy [`codesetu.example.toml`](codesetu.example.toml) to `codesetu.toml` in the project you sync.

## Safety

- Only **This PC** is written
- Atomic file writes + journal + **Undo**
- `.env` / secrets parked until you explicitly include them
- One bad file does not abort the whole apply
