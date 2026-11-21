<#
.SYNOPSIS
  Build and package READORA for Windows.

.DESCRIPTION
  Produces, in $env:DIST (default: dist):
    readora-<version>-windows-<arch>.msi            installer (WiX v5): Only for me, or Everyone
    readora-<version>-windows-<arch>-portable.zip   readora.exe + readora-cli.exe + models\ + portable.txt

  READORA is PdfCraft (https://github.com/storytold/pdfcraft) under its own name, so this is
  PdfCraft's packaging/windows/package.ps1 with the names changed. The cargo packages keep their
  upstream names (pdfcraft, pdfcraft-cli, which keeps upstream merges simple); their executables
  are shipped as readora.exe and readora-cli.exe. The file names match what READORA's built-in
  updater (Help > Check for updates) looks for on the Releases page.

  The binaries link the C runtime statically (+crt-static), so neither the MSI nor the portable
  zip needs the Visual C++ redistributable. Signing is delegated to sign.ps1 (skipped with a
  warning when no signing secrets are set).

  Needs: Rust (MSVC toolchain), curl and the network (the OCR models), the Windows SDK (rc.exe),
  and WiX v5: dotnet tool install --global wix --version 5.0.2

.EXAMPLE
  pwsh packaging/readora/package.ps1                 # build and package (x64)
  pwsh packaging/readora/package.ps1 -BuildOnly      # compile only
  pwsh packaging/readora/package.ps1 -SkipBuild      # package an existing build
#>
param(
  [ValidateSet('x64', 'x86', 'arm64')] [string] $Arch = 'x64',
  [switch] $SkipBuild,
  [switch] $BuildOnly
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Invoke-Native([string] $What, [scriptblock] $Block) {
  Write-Output "==> $What"
  & $Block
  if ($LASTEXITCODE -ne 0) { throw "$What failed with exit code $LASTEXITCODE" }
}

# The version lives in one place: [workspace.package] version in the root Cargo.toml.
$Version = $env:READORA_VERSION
if (-not $Version) {
  $inPkg = $false
  foreach ($line in Get-Content (Join-Path $Root 'Cargo.toml')) {
    if ($line -match '^\s*\[') { $inPkg = ($line.Trim() -eq '[workspace.package]'); continue }
    if ($inPkg -and $line -match '^\s*version\s*=\s*"([^"]+)"') { $Version = $Matches[1]; break }
  }
}
if (-not $Version) { throw 'could not read [workspace.package] version from Cargo.toml' }
# MSI ProductVersion is numeric (major.minor.build); pre-release tags are dropped there.
$MsiVersion = ($Version -split '-')[0]

$Target = switch ($Arch) { 'x64' { 'x86_64-pc-windows-msvc' } 'x86' { 'i686-pc-windows-msvc' } 'arm64' { 'aarch64-pc-windows-msvc' } }
$Dist = if ($env:DIST) { $env:DIST } else { Join-Path $Root 'dist' }
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
New-Item -ItemType Directory -Force -Path $Dist | Out-Null

if (-not $env:PDFCRAFT_BUILD_SHA) { $env:PDFCRAFT_BUILD_SHA = (git -C $Root rev-parse HEAD 2>$null) }
if (-not $env:PDFCRAFT_BUILD_DATE) { $env:PDFCRAFT_BUILD_DATE = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd') }

Write-Output "READORA $Version for Windows $Arch ($Target)"

if (-not $SkipBuild) {
  # Static CRT: no VC++ redistributable needed. Scoped to the target so host build scripts and
  # proc-macros are unaffected.
  $flagVar = 'CARGO_TARGET_' + ($Target.ToUpper() -replace '-', '_') + '_RUSTFLAGS'
  [Environment]::SetEnvironmentVariable($flagVar, '-C target-feature=+crt-static')
  # Fail the build (rather than warn) if the icon/VERSIONINFO can't be embedded.
  $env:PDFCRAFT_REQUIRE_WINRES = '1'
  Invoke-Native "cargo build ($Target)" { cargo build --release --locked -p pdfcraft -p pdfcraft-cli --target $Target }
}
if ($BuildOnly) { exit 0 }

$Bin = Join-Path $TargetDir "$Target\release"

# PE checks: Machine must match -Arch; the app must be a GUI program (subsystem 2, no console
# window), the CLI a console one (3) so its output reaches the terminal.
function Get-PeHeader([string] $Path) {
  $bytes = [System.IO.File]::ReadAllBytes($Path)
  $pe = [BitConverter]::ToInt32($bytes, 0x3C)
  return @{ Machine = [BitConverter]::ToUInt16($bytes, $pe + 4); Subsystem = [BitConverter]::ToUInt16($bytes, $pe + 0x5C) }
}
$Machine = switch ($Arch) { 'x64' { 0x8664 } 'x86' { 0x14C } 'arm64' { 0xAA64 } }
foreach ($check in @(@('pdfcraft.exe', 2), @('pdfcraft-cli.exe', 3))) {
  $h = Get-PeHeader (Join-Path $Bin $check[0])
  if ($h.Machine -ne $Machine) { throw "$($check[0]) is for machine 0x$('{0:X}' -f $h.Machine), expected 0x$('{0:X}' -f $Machine) ($Arch)" }
  if ($h.Subsystem -ne $check[1]) { throw "$($check[0]) has PE subsystem $($h.Subsystem), expected $($check[1])" }
  Write-Output "ok $($check[0]): $Arch, PE subsystem $($h.Subsystem)"
}

# ---- stage: readora.exe, readora-cli.exe, licences, OCR models -------------------------------
$Stage = Join-Path $TargetDir "readora-package\$Arch"
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item (Join-Path $Bin 'pdfcraft.exe') (Join-Path $Stage 'readora.exe')
Copy-Item (Join-Path $Bin 'pdfcraft-cli.exe') (Join-Path $Stage 'readora-cli.exe')
foreach ($f in 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE') {
  Copy-Item (Join-Path $Root $f) $Stage
}
# The PDF document icon embeds Lucide artwork: its ISC notice travels with the binaries.
Copy-Item (Join-Path $Root 'assets\app-icon\LICENSE-document.txt') $Stage

& (Join-Path $PSScriptRoot 'sign.ps1') (Join-Path $Stage 'readora.exe') (Join-Path $Stage 'readora-cli.exe')

# OCR models: the app looks in models\ beside readora.exe. `cargo xtask models` fetches every
# ATTRIBUTION.toml `kind = "model"` file (verified by SHA-256) with its licence and ATTRIBUTION.txt.
$Models = Join-Path $Stage 'models'
New-Item -ItemType Directory -Force -Path $Models | Out-Null
Push-Location $Root
try { Invoke-Native 'cargo xtask models' { cargo xtask models $Models } } finally { Pop-Location }
Remove-Item -Force (Join-Path $Models '*.part') -ErrorAction SilentlyContinue
if (-not (Test-Path (Join-Path $Models 'ATTRIBUTION.txt')) -or -not (Get-ChildItem (Join-Path $Models '*.LICENCE.txt'))) {
  throw "no OCR models in $Models after cargo xtask models"
}

# Tesseract and its Sinhala, Tamil and English data (get-tesseract.ps1), in tesseract\ beside
# readora.exe: Recognize Text reads Sinhala and Tamil with it.
$WixTesseract = @()
$Tesseract = Join-Path $Stage 'tesseract'
if ($env:READORA_TESSERACT_DIR) {
  if (-not (Test-Path (Join-Path $env:READORA_TESSERACT_DIR 'tesseract.exe'))) { throw "READORA_TESSERACT_DIR=$env:READORA_TESSERACT_DIR has no tesseract.exe" }
  Copy-Item -Recurse $env:READORA_TESSERACT_DIR $Tesseract
  $WixTesseract = @('-d', "TesseractDir=$Tesseract")
  Write-Output "with Tesseract from $env:READORA_TESSERACT_DIR"
} else {
  Write-Warning 'READORA_TESSERACT_DIR is not set: no bundled Tesseract (Sinhala and Tamil OCR then need an installed one)'
}

# ---- font notices (only for builds made with CRAFT_FONTS_DIR; READORA's normally aren't) -------
$Docs = Join-Path $Stage 'font-licences'
New-Item -ItemType Directory -Force -Path $Docs | Out-Null
$HasCraftFontLicences = 0
if ($env:CRAFT_FONTS_DIR) {
  . (Join-Path $PSScriptRoot 'font-notices.ps1')
  $fontNotices = @(Get-CraftFontNotices $env:CRAFT_FONTS_DIR (Test-Path Env:CRAFT_FONTS_REQUIRED))
  foreach ($ofl in $fontNotices) {
    Copy-Item -LiteralPath $ofl.FullName -Destination (Join-Path $Docs "OFL-$($ofl.Directory.Name).txt")
    $HasCraftFontLicences = 1
  }
}

# ---- MSI ---------------------------------------------------------------------------------------
$Msi = Join-Path $Dist "readora-$Version-windows-$Arch.msi"
Invoke-Native 'wix build' {
  wix build (Join-Path $PSScriptRoot 'readora.wxs') -arch $Arch `
    (Join-Path $PSScriptRoot 'installer-ui.wxs') `
    -d "Version=$MsiVersion" -d "BinDir=$Stage" -d "ModelsDir=$Models" -d "IconPath=$(Join-Path $Root 'assets\app-icon\pdfcraft.ico')" `
    -d "DocsDir=$Docs" -d "HasCraftFontLicences=$HasCraftFontLicences" @WixTesseract `
    -o $Msi
}
# Inspect the built MSI, not just the XML. In a child process, so its Windows Installer database
# handle is gone before anything else opens the MSI.
Invoke-Native 'test-msi' {
  & (Get-Process -Id $PID).Path -NoProfile -File (Join-Path $PSScriptRoot 'test-msi.ps1') $Msi `
    -CraftFontLicences $HasCraftFontLicences -ExpectedFontNoticesDirectory $Docs
}
& (Join-Path $PSScriptRoot 'check-install-policy.ps1') -MsiPath $Msi
Remove-Item -Force -ErrorAction SilentlyContinue ([IO.Path]::ChangeExtension($Msi, '.wixpdb'))
& (Join-Path $PSScriptRoot 'sign.ps1') $Msi

# ---- portable zip ------------------------------------------------------------------------------
# One top-level folder, readora-<version>-windows-<arch>-portable\, as the updater expects.
$Portable = Join-Path $TargetDir "readora-package\readora-$Version-windows-$Arch-portable"
Remove-Item -Recurse -Force $Portable -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Portable | Out-Null
Copy-Item (Join-Path $Stage '*.exe') $Portable
foreach ($f in 'NOTICE', 'LICENSE-document.txt', 'LICENSE-MIT', 'LICENSE-APACHE') {
  Copy-Item (Join-Path $Stage $f) $Portable
}
Copy-Item -Recurse $Models (Join-Path $Portable 'models')
if (Test-Path $Tesseract) { Copy-Item -Recurse $Tesseract (Join-Path $Portable 'tesseract') }
Copy-Item (Join-Path $PSScriptRoot 'README-READORA.txt') (Join-Path $Portable 'README.txt')
Copy-Item (Join-Path $PSScriptRoot 'Create Desktop Shortcut.bat') $Portable
if ($HasCraftFontLicences -eq 1) {
  Copy-Item (Join-Path $Docs 'OFL-*.txt') $Portable
}
# portable.txt beside readora.exe switches on portable mode: settings, logs and recovery files go
# to READORAData\ next to the exe instead of %APPDATA%\READORA.
Copy-Item (Join-Path $PSScriptRoot 'portable.txt') $Portable
$Zip = Join-Path $Dist "readora-$Version-windows-$Arch-portable.zip"
Remove-Item -Force $Zip -ErrorAction SilentlyContinue
Compress-Archive -Path $Portable -DestinationPath $Zip

$HostArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
if ($Arch -ne 'arm64' -or $HostArch -eq 'arm64') {
  Invoke-Native 'readora-cli --version' { & (Join-Path $Stage 'readora-cli.exe') --version }
}
Get-Item $Msi, $Zip | Format-Table Name, Length
