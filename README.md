<p align="center">
  <img src="assets/app-icon/hicolor/256x256/apps/ai.storyteller.pdfcraft.png" width="128" height="128" alt="READORA icon: a page with an R on a warm red tile">
</p>

<h1 align="center">READORA</h1>

<p align="center">
  <b>A smooth PDF reader for Windows, by Imal Madhusanka.</b><br>
  Read, search, organize and sign PDFs in a fast, native app. Offline, no account, no telemetry.
</p>

<p align="center">
  <a href="https://github.com/imalmadhusankaphotography-creator/READORA/releases/latest"><b>⬇ Download for Windows</b></a>
</p>

---

## Download

Get the latest build from the [**Releases page**](https://github.com/imalmadhusankaphotography-creator/READORA/releases/latest):

| File | |
|---|---|
| `readora-<version>-windows-x64.msi` | **Installer**: READORA in the Start menu, on the desktop and under "Open with" for PDF files |
| `readora-<version>-windows-x64-portable.zip` | **Portable**: unzip anywhere (a pen drive works too) and run `readora.exe` |

The files are not code-signed, so the first time Windows may say it protected your PC: choose
**More info ▸ Run anyway**. To open every PDF with READORA: right-click a PDF ▸ **Open with** ▸
**Choose another app** ▸ **READORA** ▸ **Always**.

### සිංහලෙන්

- Releases page එකෙන් **`.msi`** එක download කරලා double-click කරන්න. Install වුණාම Start menu එකේ සහ desktop එකේ READORA එනවා.
- හැම PDF එකක්ම READORA එකෙන් open වෙන්න: PDF එකක් right-click › **Open with** › **Choose another app** › **READORA** › **Always**.
- අලුත් version එකක් ආවද බලන්න: app එකේ **Help › Check for updates**.

## READORA for Android

The phone version (`mobile/`) scans paper to PDF with the camera and reads PDFs. Get the newest
`READORA-1.0.x.apk` from the [Releases page](https://github.com/imalmadhusankaphotography-creator/READORA/releases)
(the `android-v1.0.x` releases) and open it on the phone to install.

- **Scan**: Google's ML Kit document scanner: finds the page edges by itself, crops and straightens,
  filters, removes stains and fingers, many pages into one PDF; photos from the gallery work too.
- Scans are saved in the phone's **Documents › READORA** folder. Share them (WhatsApp, email), save a
  copy to **Downloads**, or save the pages as **JPG** images.
- **Read**: open any PDF (or "Open with › READORA" from a file manager): smooth scrolling, pinch
  zoom, double-tap zoom, search, night mode, go to page, password-protected PDFs.

It is a Capacitor app (one `www/index.html`, pdf.js for the pages) built by
`.github/workflows/readora-android.yml` and signed with the fixed key in `mobile/android` so every
new APK installs over the last one.

## What it does

- **Reads beautifully.** Sharp text at any zoom (large pages render in tiles), complex scripts,
  Sinhala, vertical Japanese, colour emoji, transparency. Continuous, single page, two-up,
  full screen and a distraction-free Read mode; light and dark themes.
- **Stays smooth.** Native Rust, drawing on the GPU with DirectX 12 (OpenGL as a fallback). Damaged
  files are repaired, and every page renders on its own, so one bad page can't take the app down.
- **Finds things.** Search as you type, bookmarks, thumbnails, page labels.
- **Scans.** File ▸ Create PDF from scanner… scans paper pages with your scanner (Windows'
  own scanner window, any scanner with a Windows driver) straight into a PDF.
- **Reads scans in Sinhala, Tamil and English.** Recognize text (OCR) turns scanned pages into
  searchable, copyable text; Sinhala and Tamil are read by the Tesseract engine that comes with
  READORA.
- **Exports to Word and Excel.** File ▸ Export to Word… (.docx) and Export to Excel… (.xlsx: the
  tables on each page become cells, numbers stay numbers), plus PowerPoint, HTML and RTF.
- **Edits.** Edit text and images, add text, images, links, comments, headers and watermarks.
- **Organizes pages.** Combine, split, extract, rotate and reorder.
- **Does the rest.** Forms, digital signatures, password-protected PDFs, layers, attachments and
  printing.
- **Updates itself.** Help ▸ Check for updates finds the newest READORA release here.

## Build it yourself

GitHub Actions builds every push to `main` (`.github/workflows/readora-windows.yml`), starts the
app once with a sample PDF to check it opens, and publishes the installer and the portable zip on
the Releases page.

On a Windows PC with [Rust](https://rustup.rs) (1.95 or later, MSVC toolchain) and WiX v5
(`dotnet tool install --global wix --version 5.0.2`):

```powershell
git clone https://github.com/imalmadhusankaphotography-creator/READORA
cd READORA
cargo run --release -p pdfcraft -- some.pdf          # just run it
pwsh packaging/readora/package.ps1                   # the .msi and the portable .zip, in dist\
```

The Rust packages keep PdfCraft's names (`pdfcraft`, `pdfcraft-cli`) so that new PdfCraft releases
merge cleanly; the packages ship them as `readora.exe` and `readora-cli.exe`.
`packaging/readora/UPDATING.md` explains how to bring in a new PdfCraft release, and
`packaging/readora/make_logo_assets.py` remakes every icon from a new logo.

## Credits and licence

READORA is built on [**PdfCraft**](https://github.com/storytold/pdfcraft), the open-source PDF
workbench written in Rust by the PdfCraft contributors. READORA is an independent version of it and
is not made or endorsed by the PdfCraft or ArtCraft teams; the ArtCraft name and logos, which
PdfCraft's brand terms reserve, are not included.

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option, like
PdfCraft. Third-party material is listed in [NOTICE](NOTICE) and [ATTRIBUTION.md](ATTRIBUTION.md).
The READORA name and app icon are Imal Madhusanka's.
