READORA - a smooth PDF reader for Windows, by Imal Madhusanka
=============================================================

Start
  Installer (.msi): READORA is in the Start menu and on the desktop, and under "Open with" for
  PDF files. To open every PDF with it: right-click a PDF > Open with > Choose another app >
  READORA > Always.
  Portable (.zip): unzip the folder anywhere (a pen drive works too) and double-click
  readora.exe. "Create Desktop Shortcut.bat" adds READORA to the desktop and the Start menu.

What it does
  Read: continuous, single page, two-up and full-screen Read mode, light and dark themes, sharp
  text at any zoom, search as you type, bookmarks, thumbnails.
  Organize: combine, split, extract, rotate and reorder pages.
  Comments, forms, signatures, password-protected PDFs and OCR (scanned pages to searchable text).
  Sinhala text shows with the Windows Sinhala font (Nirmala UI / Iskoola Pota).

Your settings
  Installed: %APPDATA%\READORA. Portable: the READORAData folder next to readora.exe (delete
  portable.txt to use %APPDATA%\READORA).

If READORA doesn't open
  READORA draws with DirectX 12 and falls back to OpenGL by itself. To force OpenGL, start it
  from PowerShell in its folder:
      $env:PDFCRAFT_RENDERER = "gl"
      & .\readora.exe
  The log file pdfcraft.log (in %APPDATA%\READORA\data\logs; portable: READORAData\logs)
  says what went wrong.

Updates
  Help > Check for updates looks for a newer READORA on
  https://github.com/imalmadhusankaphotography-creator/READORA/releases

readora-cli.exe is the command-line tool (readora-cli --help): batch conversion, OCR, page tools.

READORA is built on PdfCraft (https://github.com/storytold/pdfcraft) by the PdfCraft
contributors, MIT OR Apache-2.0. It is not made or endorsed by the PdfCraft or ArtCraft teams.
Licences: LICENSE-MIT, LICENSE-APACHE, NOTICE, LICENSE-document.txt, models\ATTRIBUTION.txt.
