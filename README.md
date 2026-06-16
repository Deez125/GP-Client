# GP Client

A custom, standalone Minecraft launcher for GP Client supported servers. Sign
in, install the modpack, and play — no official launcher needed.

---

## What it does

- **Launches Minecraft on its own** — no official launcher required.
- **Microsoft sign-in** that keeps you logged in between sessions.
- **Separate installs per version**, while your vanilla worlds, resource packs,
  and shaderpacks are shared automatically.
- **Installs and updates the server's modpack** for you, and never touches mods
  you add yourself.
- **Optional mods picker** — browse extras by category with preview images and
  toggle the ones you want.
- **Your skin, in 3D** — see your character rendered, with a skin library.
- **Pick your server** from the sidebar.

---

## Built with

[Tauri 2](https://tauri.app) (Rust + web frontend), React, and TypeScript.
Targets Minecraft 26.1.2 (requires Java 25). Windows is the primary platform.

---

## Running it

```bash
cd launcher
npm install
npm run tauri dev
```

Build a release with `npm run tauri build`. You'll need Node, Rust, and the
standard [Tauri prerequisites](https://tauri.app/start/prerequisites/).

---

## Layout

```
GP Client/
├── launcher/     # the app — React frontend (src/) + Rust backend (src-tauri/)
└── reference/    # design sources & notes (not committed)
```
