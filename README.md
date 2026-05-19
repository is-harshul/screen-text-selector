# Text Extractor

A cross-platform Tauri desktop utility that extracts text from any region of your screen — including text inside images, videos, and other non-copyable interfaces — using local Tesseract OCR.

Press the global hotkey, drag a rectangle, and the recognised text lands on your clipboard.

> Status: Tauri 2.x · macOS / Linux / Windows · 100% local OCR (no cloud calls)

### Demo

https://github.com/user-attachments/assets/8a52013b-8141-4103-b866-d448baca3db9

---

## Table of contents

- [How it works](#how-it-works)
- [Quick start (users) | Download](#quick-start-users)
- [Quick start (developers)](#quick-start-developers)
- [Building distributable installers](#building-distributable-installers)
- [Platform notes](#platform-notes)
- [Project layout](#project-layout)
- [Architecture notes](#architecture-notes)
- [Roadmap](#roadmap)

---

## How it works

1. App runs in the system tray (no main window).
2. Global hotkey `Ctrl+Shift+T` (Windows/Linux) / `Cmd+Shift+T` (macOS) triggers a screen capture.
3. A transparent fullscreen overlay opens for selection.
4. The selected region is cropped, preprocessed (upscaled + grayscaled), and run through Tesseract.
5. Result is written to the clipboard with a notification toast.

---

## Quick start (users)

> Just want to use the app? Grab the installer for your OS, install, launch — done.

### Download

Latest installers: [**Releases page**](https://github.com/is-harshul/text-extractor/releases/latest) · current **v0.1.2**.

| OS | Installer | Direct download |
| --- | --- | --- |
| macOS (Apple silicon — M1/M2/M3) | `Text Extractor_0.1.0_aarch64.dmg` | [⬇ download](https://github.com/is-harshul/text-extractor/releases/latest/download/Text.Extractor_0.1.0_aarch64.dmg) |
| macOS (Intel) | `Text Extractor_0.1.0_x64.dmg` | _coming in next release — CI runner backlog_ |
| Windows (installer) | `Text Extractor_0.1.0_x64-setup.exe` | [⬇ download](https://github.com/is-harshul/text-extractor/releases/latest/download/Text.Extractor_0.1.0_x64-setup.exe) |
| Windows (MSI) | `Text Extractor_0.1.0_x64_en-US.msi` | [⬇ download](https://github.com/is-harshul/text-extractor/releases/latest/download/Text.Extractor_0.1.0_x64_en-US.msi) |
| Linux (.deb) | `Text Extractor_0.1.0_amd64.deb` | [⬇ download](https://github.com/is-harshul/text-extractor/releases/latest/download/Text.Extractor_0.1.0_amd64.deb) |
| Linux (.AppImage) | `Text Extractor_0.1.0_amd64.AppImage` | [⬇ download](https://github.com/is-harshul/text-extractor/releases/latest/download/Text.Extractor_0.1.0_amd64.AppImage) |

> Direct-download links hit GitHub's `/releases/latest/download/<filename>` redirect — auto-tracks the newest release. GitHub stores asset filenames with `.` in place of spaces, so the URL has `Text.Extractor_...` while the file you save is named `Text Extractor_...`.
>
> If a link 404s after a version bump, the version segment in the filename changed (e.g. `0.1.0` → `0.2.0`). Open the [Releases page](https://github.com/is-harshul/text-extractor/releases/latest) and grab the file by hand.


### macOS

1. Download `Text Extractor_<version>_universal.dmg` (or the arch-specific build) from your distributor.
2. Open the DMG, drag **Text Extractor** to Applications.
3. **First launch:** right-click the app → **Open** → **Open**. (The app is unsigned; this bypasses Gatekeeper one time.)
   Alternative one-liner:
   ```bash
   xattr -dr com.apple.quarantine "/Applications/Text Extractor.app"
   ```
4. Grant **Screen Recording** permission when macOS asks (System Settings → Privacy & Security → Screen Recording). Without this, captures are black.
5. Press **`Cmd+Shift+T`** anywhere → drag a rectangle → text is on your clipboard.

### Linux

1. Install the `.deb` (Debian/Ubuntu): `sudo dpkg -i text-extractor_<version>_amd64.deb`
   Or run the `.AppImage`: `chmod +x Text\ Extractor_*.AppImage && ./Text\ Extractor_*.AppImage`
2. Press **`Ctrl+Shift+T`** → drag → paste.

### Windows

1. Run the `.msi` or `.exe` installer.
2. Press **`Ctrl+Shift+T`** → drag → paste.

---

## Quick start (developers)

### Prerequisites

- [Rust](https://rustup.rs/) (1.77+)
- [Node.js](https://nodejs.org/) (18+)
- Platform-specific Tauri prerequisites: <https://v2.tauri.app/start/prerequisites/>

### Tesseract (system dependency)

The Rust `tesseract` crate links to libtesseract.

| Platform | Install |
| --- | --- |
| **macOS** | `brew install tesseract leptonica` |
| **Debian/Ubuntu** | `sudo apt install libtesseract-dev libleptonica-dev clang` |
| **Fedora** | `sudo dnf install tesseract-devel leptonica-devel clang` |
| **Arch** | `sudo pacman -S tesseract leptonica clang` |
| **Windows** | Install [UB Mannheim Tesseract](https://github.com/UB-Mannheim/tesseract/wiki), add to `PATH`. May need `TESSDATA_PREFIX` pointing at the `tessdata` directory. |

> When shipping to end users, **vendor** the Tesseract binaries inside your installer rather than asking users to install them.

### Setup

```bash
npm install
```

If `src-tauri/icons/` is empty, generate icons from a 1024×1024 PNG:

```bash
npm run tauri icon path/to/source.png
```

### Dev mode

```bash
npm run tauri dev
```

App launches into the system tray. Press the hotkey to trigger capture.

---

## Building distributable installers

A helper script wraps `tauri build` and collects every installer into `release/` so you can zip and send to friends.

### One-liners

```bash
npm run dist           # macOS: universal DMG (arm64 + Intel) — slowest, widest reach
npm run dist:host      # current arch only — fastest, recommended for testing
npm run dist:arm64     # Apple silicon only
npm run dist:x86_64    # Intel only
```

On Linux the script auto-builds `.deb` + `.AppImage`. On Windows it auto-builds `.msi` + `.exe` (NSIS).

### What you get

```
release/
└── Text Extractor_0.1.0_universal.dmg     # macOS
    Text Extractor_0.1.0_amd64.deb         # Linux
    text-extractor_0.1.0_amd64.AppImage    # Linux
    Text Extractor_0.1.0_x64_en-US.msi     # Windows
    Text Extractor_0.1.0_x64-setup.exe     # Windows
```

Send the file → friend installs → done.

### What the script does

[scripts/build-installer.sh](scripts/build-installer.sh):

1. Verifies `node`, `npm`, `cargo` are installed.
2. Runs `npm install` if `node_modules/` is missing.
3. On macOS, `rustup target add`s required arches automatically.
4. Calls `npx tauri build` with the right `--bundles` for the host OS.
5. Copies every produced installer into `release/`.

### Manual build (no script)

```bash
npm run tauri build
```

Bundles land in `src-tauri/target/release/bundle/`.

### Publishing to GitHub Releases

Two paths — pick whichever fits.

#### A. Local build, then publish (fast, single OS)

Build on your machine, attach DMG/installer to a GitHub Release in one shot:

```bash
# bumps no version — uses current package.json version as tag
npm run dist:publish
```

What it does:
1. Runs the normal `npm run dist` build (universal macOS DMG by default).
2. Reads `version` from [package.json](package.json), tags `v<version>`, pushes tag.
3. `gh release create` (or `gh release upload --clobber` if tag already exists) attaches everything in `release/`.

Override target arch:

```bash
PUBLISH=1 bash scripts/build-installer.sh host    # current arch only
PUBLISH=1 bash scripts/build-installer.sh arm64
```

Requires: `gh auth login` once.

#### B. CI builds for all OSes on tag push (recommended)

[.github/workflows/release.yml](.github/workflows/release.yml) builds **macOS arm64 + macOS Intel + Linux (.deb/.AppImage) + Windows (.msi/.exe)** in parallel, then publishes a single GitHub Release with all artifacts attached.

Trigger by pushing a tag:

```bash
# bump version in package.json + src-tauri/tauri.conf.json + src-tauri/Cargo.toml first
git commit -am "release v0.2.0"
git tag v0.2.0
git push origin main v0.2.0
```

Or manually from the **Actions** tab → **Release** → **Run workflow** → enter tag.

Friends download from `https://github.com/is-harshul/text-extractor/releases/latest`.

> First run takes ~15–25 min (cold Rust + Tesseract build on 4 OSes). Subsequent runs cache and finish in ~5–10 min.
>
> Windows job uses `vcpkg` for Tesseract. If the build fails on Windows, the most common fix is regenerating `vcpkg` cache — see [tauri-apps/tauri#windows-tesseract](https://v2.tauri.app/distribute/windows-installer/) docs.

### Code signing (optional, recommended for wider distribution)

Without signing, friends will hit OS warnings ("unidentified developer" / SmartScreen). To skip the warnings:

- **macOS:** Apple Developer ID ($99/yr). Configure `bundle.macOS.signingIdentity` in [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json) and set `APPLE_ID` / `APPLE_PASSWORD` for notarization.
- **Windows:** EV or OV code-signing certificate. Configure `bundle.windows.certificateThumbprint`.
- **Linux:** no signing required, but consider hosting the `.deb` in a PPA / providing a `.repo`.

See <https://v2.tauri.app/distribute/> for full guides.

---

## Platform notes

### macOS
- First launch requires **Screen Recording** permission (System Settings → Privacy & Security → Screen Recording). Without it `xcap` returns a black image.
- Unsigned builds trigger Gatekeeper. Right-click → Open works, or strip quarantine: `xattr -dr com.apple.quarantine "/Applications/Text Extractor.app"`.

### Linux
- **X11:** works out of the box.
- **Wayland:** `xcap` uses the `xdg-desktop-portal` protocol, which prompts the user to grant screen access *every capture session*. Wayland design decision — no way around it. Recommend X11 to users for a smoother experience.

### Windows
- Smoothest of the three. Make sure your installer bundles `tesseract.dll`, `leptonica.dll`, and the `tessdata` folder (English language data ≈ 15 MB).

---

## Project layout

```
text-extractor/
├── src/                       # React overlay UI
│   ├── App.tsx                # Selection rectangle, masks, hint UI
│   ├── main.tsx               # React entry
│   └── styles.css             # Overlay styles
├── src-tauri/
│   ├── src/
│   │   ├── main.rs            # Desktop entry point
│   │   ├── lib.rs             # Tauri setup, tray, hotkey, commands
│   │   ├── capture.rs         # xcap-based screen capture
│   │   └── ocr.rs             # Tesseract wrapper + preprocessing
│   ├── capabilities/
│   │   └── default.json       # Tauri 2 permissions
│   ├── Cargo.toml
│   └── tauri.conf.json
├── scripts/
│   └── build-installer.sh     # Distributable-installer builder
├── index.html
├── package.json
├── tsconfig.json
└── vite.config.ts
```

---

## Architecture notes

- **Capture is taken before the overlay shows** so the user selects against a frozen, pre-captured image. Capturing *after* selection causes flicker and includes the overlay in the screenshot.
- **OCR runs on a blocking task** (`tokio::task::spawn_blocking`) so it doesn't stall the Tauri event loop.
- **The overlay window is hidden, not closed**, between captures. Recreating windows is slow on macOS and would add noticeable latency.
- **Scale factors are computed in the renderer** (`physicalWidth / window.innerWidth`) to handle HiDPI/Retina displays. On mixed-DPI multi-monitor setups this can drift — v2 fix is to capture the cursor's monitor specifically.

---

## Roadmap

- Multi-monitor support (capture the monitor under the cursor)
- Language picker (Tesseract supports 100+ languages)
- Selection refinement before commit (drag handles to adjust the rectangle)
- Optional cloud OCR (Google Vision) for higher accuracy
- Capture history (last N extractions, accessible from tray)
- Settings window (custom hotkey, OCR language, preprocessing toggles)

---

## License

TBD — add a `LICENSE` file before public distribution.
