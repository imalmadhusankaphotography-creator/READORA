//! Create ▸ From scanner (READORA): scan paper pages with the computer's scanner and make a PDF of
//! them, optionally reading their text (OCR) right away, in English, Sinhala or Tamil.
//!
//! On Windows each page is scanned through Windows' own scanner window (Windows Image
//! Acquisition, the `WIA.CommonDialog` scripting object), started in a PowerShell process with a
//! fixed script: no document data reaches it, and READORA needs no unsafe code or scanner driver
//! of its own. Every scanner with a Windows driver works, with its own preview, colour and
//! resolution settings. The scan runs off the interface thread; the dialog shows the pages as they
//! arrive. Other platforms say that scanning needs Windows.

use crate::PdfCraftApp;
use crate::widgets;

/// A scanned page: its JPEG bytes and the resolution it was scanned at (dots per inch).
pub(crate) struct ScannedPage {
    pub(crate) jpeg: Vec<u8>,
    pub(crate) dpi: f64,
}

/// What one scan brought back.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) enum ScanResult {
    Page(ScannedPage),
    /// The user closed the scanner window without scanning.
    Cancelled,
    /// No scanner is connected (or none has a Windows driver).
    NoScanner,
    Failed(String),
}

/// The Scan to PDF dialog's state.
pub struct ScanJob {
    pub(crate) pages: Vec<ScannedPage>,
    /// The running scan's answer, while it runs.
    waiting: Option<std::sync::mpsc::Receiver<ScanResult>>,
    /// What went wrong with the last scan, for the dialog.
    pub(crate) message: Option<String>,
    /// Read the text of the pages (OCR) once the PDF is made.
    pub(crate) ocr: bool,
    /// The OCR language (a Recognize Text language code).
    pub(crate) language: String,
}

impl Default for ScanJob {
    fn default() -> Self {
        ScanJob { pages: Vec::new(), waiting: None, message: None, ocr: true, language: "en".into() }
    }
}

impl ScanJob {
    pub(crate) fn scanning(&self) -> bool {
        self.waiting.is_some()
    }
}

/// The PowerShell script that scans one page: Windows' scanner window, then the image as JPEG at
/// `READORA_SCAN_OUT`, and its resolution on stdout (`DPI=300`). Exit 2: cancelled; 3: no scanner.
#[cfg(windows)]
const SCAN_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$out = $env:READORA_SCAN_OUT
$bmp = '{B96B3CAB-0728-11D3-9D7B-0000F81EF32E}'
$jpeg = '{B96B3CAE-0728-11D3-9D7B-0000F81EF32E}'
try {
  $dialog = New-Object -ComObject WIA.CommonDialog
  # Scanner (1), the device's default intent and bias, any format, pick a device only when there
  # are several, Windows' own scan window, a cancel returns nothing.
  $image = $dialog.ShowAcquireImage(1, 0, 0, $bmp, $false, $true, $false)
} catch {
  if (('{0:X8}' -f $_.Exception.HResult) -eq '80210015') { exit 3 }
  [Console]::Error.WriteLine($_.Exception.Message)
  exit 1
}
if ($null -eq $image) { exit 2 }
try {
  $dpi = [double]$image.HorizontalResolution
  if ($image.FormatID -ne $jpeg) {
    $process = New-Object -ComObject WIA.ImageProcess
    $process.Filters.Add($process.FilterInfos.Item('Convert').FilterID)
    $process.Filters.Item(1).Properties.Item('FormatID').Value = $jpeg
    $process.Filters.Item(1).Properties.Item('Quality').Value = 92
    $image = $process.Apply($image)
  }
  $image.SaveFile($out)
  [Console]::Out.WriteLine("DPI=$dpi")
} catch {
  [Console]::Error.WriteLine($_.Exception.Message)
  exit 1
}
exit 0
"#;

/// Largest scanned page READORA takes (a 600 dpi A3 colour JPEG is far below this).
#[cfg(windows)]
const MAX_SCAN_BYTES: u64 = 256 * 1024 * 1024;

/// Scan one page (blocks until the scanner window closes).
#[cfg(windows)]
fn scan_page() -> ScanResult {
    use std::os::windows::process::CommandExt as _;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("readora-scan-{}-{n}", std::process::id()));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return ScanResult::Failed(format!("couldn't make a folder for the scan: {e}"));
    }
    let out = dir.join("page.jpg");
    let system_root = std::env::var_os("SystemRoot").map_or_else(|| std::path::PathBuf::from(r"C:\Windows"), std::path::PathBuf::from);
    let powershell = system_root.join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let output = std::process::Command::new(&powershell)
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", SCAN_SCRIPT])
        .env("READORA_SCAN_OUT", &out)
        .stdin(std::process::Stdio::null())
        // No console window; the scanner window still shows.
        .creation_flags(0x0800_0000)
        .output();
    let result = match output {
        Err(e) => ScanResult::Failed(format!("Windows PowerShell couldn't start: {e}")),
        Ok(o) => match o.status.code() {
            Some(0) => {
                let dpi = String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .find_map(|l| l.trim().strip_prefix("DPI=").and_then(|v| v.trim().parse::<f64>().ok()))
                    .filter(|d| d.is_finite() && (50.0..=4800.0).contains(d))
                    .unwrap_or(300.0);
                match std::fs::metadata(&out) {
                    Ok(m) if m.len() > MAX_SCAN_BYTES => ScanResult::Failed("the scanned page is too large".into()),
                    Ok(_) => match std::fs::read(&out) {
                        Ok(jpeg) if !jpeg.is_empty() => ScanResult::Page(ScannedPage { jpeg, dpi }),
                        Ok(_) => ScanResult::Failed("the scanner sent an empty page".into()),
                        Err(e) => ScanResult::Failed(format!("couldn't read the scanned page: {e}")),
                    },
                    Err(e) => ScanResult::Failed(format!("the scanner didn't save the page: {e}")),
                }
            }
            Some(2) => ScanResult::Cancelled,
            Some(3) => ScanResult::NoScanner,
            _ => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                ScanResult::Failed(if err.is_empty() { format!("the scan stopped ({})", o.status) } else { err })
            }
        },
    };
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(not(windows))]
fn scan_page() -> ScanResult {
    ScanResult::Failed("scanning needs Windows (READORA uses Windows' own scanner window)".into())
}

impl PdfCraftApp {
    /// Create ▸ From scanner: open the Scan to PDF dialog and scan the first page.
    pub(crate) fn scan_to_pdf_dialog(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.notify_tr("Scanning needs the Windows app");
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut job = ScanJob::default();
            // The language Recognize Text last used, so a Sinhala user keeps Sinhala.
            if pdfcraft_engine::ocr::LANGUAGES.iter().any(|(c, _)| *c == self.ocr_draft.language) {
                job.language = self.ocr_draft.language.clone();
            }
            self.scan = Some(job);
            self.dialog = Some(crate::Dialog::Scan);
            self.scan_next_page();
        }
    }

    /// Start scanning one more page (in the background; the dialog shows when it arrives).
    pub(crate) fn scan_next_page(&mut self) {
        let ctx = self.ctx.clone();
        let Some(job) = self.scan.as_mut() else { return };
        if job.scanning() {
            return;
        }
        job.message = None;
        let (tx, rx) = std::sync::mpsc::channel();
        job.waiting = Some(rx);
        let spawned = std::thread::Builder::new().name("readora-scan".into()).spawn(move || {
            let result = std::panic::catch_unwind(scan_page).unwrap_or_else(|_| ScanResult::Failed("the scan stopped unexpectedly".into()));
            let _ = tx.send(result);
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
        if let Err(e) = spawned {
            job.waiting = None;
            job.message = Some(format!("couldn't start the scan: {e}"));
        }
    }

    /// Pick up a finished scan (each frame while the dialog is open).
    pub(crate) fn poll_scan(&mut self) {
        let Some(job) = self.scan.as_mut() else { return };
        let Some(rx) = job.waiting.as_ref() else { return };
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => ScanResult::Failed("the scan stopped unexpectedly".into()),
        };
        job.waiting = None;
        match result {
            ScanResult::Page(page) => job.pages.push(page),
            ScanResult::Cancelled => {}
            ScanResult::NoScanner => {
                job.message =
                    Some(crate::i18n::t("No scanner was found. Connect the scanner, turn it on, and install its Windows driver.").to_string());
            }
            ScanResult::Failed(e) => job.message = Some(e),
        }
    }

    /// Make the PDF of the scanned pages (one page each, at the size they were scanned), then read
    /// their text when asked.
    pub(crate) fn finish_scan(&mut self) {
        let Some(job) = self.scan.take() else { return };
        if job.pages.is_empty() {
            return;
        }
        let dpi = job.pages.first().map_or(300.0, |p| p.dpi);
        let images: Vec<(String, Vec<u8>)> = job.pages.into_iter().enumerate().map(|(i, p)| (format!("Scan {}.jpg", i + 1), p.jpeg)).collect();
        let before = self.views.len();
        self.create_from_images_with_resolution(images, pdfcraft_engine::ImageResolution::Dpi(dpi));
        if self.views.len() > before && job.ocr {
            self.ocr_draft.language = job.language;
            self.ocr_draft.pages = crate::ocr_ui::OcrPages::All;
            // Scans come in a little crooked or turned: straighten them before reading.
            self.ocr_draft.deskew = true;
            self.ocr_draft.auto_rotate = true;
            self.start_ocr();
        }
    }
}

/// The dialog's content. Returns (create the PDF now, close the dialog).
pub(crate) fn body(ui: &mut egui::Ui, app: &mut PdfCraftApp) -> (bool, bool) {
    app.poll_scan();
    let t = crate::theme::Tokens::get(ui.ctx());
    ui.heading(tl!("Scan to PDF"));
    ui.add_space(4.0);
    let mut scan_more = false;
    let mut create = false;
    let mut cancel = false;
    {
        let Some(job) = app.scan.as_mut() else { return (false, true) };
        if job.scanning() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tl!("Scanning… Use the scanner window to preview and scan the page."));
            });
            ui.label(egui::RichText::new(tl!("If you don't see the scanner window, look for it on the taskbar.")).color(t.text_muted));
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
        let n = job.pages.len();
        ui.label(crate::i18n::fmt(tl!("Pages scanned: {count}"), &[("count", &n.to_string())]));
        if let Some(msg) = &job.message {
            ui.label(egui::RichText::new(msg.as_str()).color(t.accent_text));
        }
        ui.add_space(8.0);
        ui.checkbox(&mut job.ocr, tl!("Recognize text after scanning (searchable, copyable text)"));
        ui.add_enabled_ui(job.ocr, |ui| {
            ui.horizontal(|ui| {
                ui.label(tl!("Text language"));
                let name = pdfcraft_engine::ocr::LANGUAGES.iter().find(|(c, _)| *c == job.language).map_or("English", |(_, n)| n);
                craft_ui::ComboBox::from_id_salt("scan-ocr-language").selected_text(tl!(name)).width(160.0).show_ui(ui, |ui| {
                    for (code, name) in pdfcraft_engine::ocr::LANGUAGES {
                        ui.selectable_value(&mut job.language, (*code).to_string(), tl!(*name));
                    }
                });
            });
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let busy = job.scanning();
            let label = if n == 0 { tl!("Scan a page") } else { tl!("Scan another page") };
            scan_more = ui.add_enabled_ui(!busy, |ui| widgets::pill_button(ui, label, n == 0)).inner.clicked();
            create = ui.add_enabled_ui(!busy && n > 0, |ui| widgets::pill_button(ui, tl!("Create PDF"), n > 0)).inner.clicked();
            cancel = widgets::pill_button(ui, tl!("Cancel"), false).clicked();
        });
    }
    if scan_more {
        app.scan_next_page();
    }
    (create, cancel)
}
