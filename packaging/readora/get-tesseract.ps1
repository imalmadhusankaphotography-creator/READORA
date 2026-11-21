<#
.SYNOPSIS
  Build the Tesseract OCR engine READORA ships, and fetch its Sinhala, Tamil and English data.

.DESCRIPTION
  READORA reads Sinhala and Tamil scans with Tesseract (https://github.com/tesseract-ocr/tesseract,
  Apache-2.0). This builds it with vcpkg as one static tesseract.exe (no DLLs, static C runtime;
  without the optional curl and libarchive features) and downloads the language data from
  tesseract-ocr/tessdata_best (Apache-2.0) at a pinned commit, each file checked by SHA-256.

  Output (-Out), copied by package.ps1 to tesseract\ beside readora.exe:
    tesseract.exe
    tessdata\eng.traineddata, sin.traineddata, tam.traineddata
    LICENSE-tessdata.txt, THIRD-PARTY-LICENSES.txt (Tesseract and every library linked into it)

  The app finds tesseract\tesseract.exe next to itself and points it at tesseract\tessdata
  (crates/ocr/src/tesseract.rs, bundled_path).

.EXAMPLE
  pwsh packaging/readora/get-tesseract.ps1 -Out tesseract-bundle
#>
param([string] $Out = 'tesseract-bundle')
$ErrorActionPreference = 'Stop'

# tessdata_best at this commit; SHA-256 of each file.
$TessdataCommit = 'e12c65a915945e4c28e237a9b52bc4a8f39a0cec'
$Languages = [ordered]@{
  eng = '8280aed0782fe27257a68ea10fe7ef324ca0f8d85bd2fd145d1c2b560bcb66ba'
  sin = '1d95b36e935c8c12cc890684fad15bd3ad8d904c3a6ffa2fccb81658eabd8454'
  tam = '4b9ce85987f629dd31eaf87443a1646452a43cdf91fcf05e017382ad595dcb9e'
}

function Invoke-Native([string] $What, [scriptblock] $Block) {
  Write-Output "==> $What"
  & $Block
  if ($LASTEXITCODE -ne 0) { throw "$What failed with exit code $LASTEXITCODE" }
}

$vcpkgRoot = if ($env:VCPKG_INSTALLATION_ROOT) { $env:VCPKG_INSTALLATION_ROOT } elseif ($env:VCPKG_ROOT) { $env:VCPKG_ROOT } else { 'C:\vcpkg' }
$vcpkg = Join-Path $vcpkgRoot 'vcpkg.exe'
if (-not (Test-Path $vcpkg)) { throw "vcpkg not found at $vcpkg (set VCPKG_ROOT)" }
$triplet = 'x64-windows-static'
Invoke-Native 'vcpkg install tesseract' { & $vcpkg install "tesseract[core]:$triplet" --clean-after-build }

$installed = Join-Path $vcpkgRoot "installed\$triplet"
$exe = Join-Path $installed 'tools\tesseract\tesseract.exe'
if (-not (Test-Path $exe)) { throw "vcpkg built no tesseract.exe at $exe" }

New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$data = Join-Path $Out 'tessdata'
New-Item -ItemType Directory -Force -Path $data | Out-Null
Copy-Item $exe $Out
# Anything else vcpkg put beside the tool (a static build should have nothing; copied if it does).
Get-ChildItem (Split-Path $exe) -Filter *.dll -ErrorAction SilentlyContinue | Copy-Item -Destination $Out

# Tesseract's own config files (configs\tsv, tessconfigs\…, pdf.ttf), which vcpkg installs in
# share\tessdata: output formats chosen by name (`tesseract in.png out tsv`) read them.
$shared = Join-Path $installed 'share\tessdata'
if (Test-Path $shared) {
  Get-ChildItem $shared | Where-Object { $_.Name -notlike '*.traineddata' } | Copy-Item -Recurse -Destination $data -Force
}
if (-not (Test-Path (Join-Path $data 'configs\tsv'))) {
  New-Item -ItemType Directory -Force -Path (Join-Path $data 'configs') | Out-Null
  Set-Content -Encoding ascii -Path (Join-Path $data 'configs\tsv') -Value 'tessedit_create_tsv 1'
}

foreach ($lang in $Languages.Keys) {
  $file = Join-Path $data "$lang.traineddata"
  curl.exe -fsSL --retry 3 -o $file "https://raw.githubusercontent.com/tesseract-ocr/tessdata_best/$TessdataCommit/$lang.traineddata"
  if ($LASTEXITCODE -ne 0) { throw "downloading $lang.traineddata failed" }
  $hash = (Get-FileHash $file -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($hash -ne $Languages[$lang]) { throw "$lang.traineddata has SHA-256 $hash, expected $($Languages[$lang])" }
}
curl.exe -fsSL --retry 3 -o (Join-Path $Out 'LICENSE-tessdata.txt') "https://raw.githubusercontent.com/tesseract-ocr/tessdata_best/$TessdataCommit/LICENSE"
if ($LASTEXITCODE -ne 0) { throw 'downloading the tessdata licence failed' }

# The licence of every package vcpkg built into tesseract.exe.
$notice = New-Object System.Text.StringBuilder
[void]$notice.AppendLine('Third-party software in tesseract.exe (built with vcpkg, triplet x64-windows-static)')
[void]$notice.AppendLine('Language data: tesseract-ocr/tessdata_best ' + $TessdataCommit + ' (Apache-2.0, LICENSE-tessdata.txt)')
[void]$notice.AppendLine('')
$list = & $vcpkg list --x-full-desc 2>$null | Where-Object { $_ -match ":$triplet" }
[void]$notice.AppendLine('Packages:')
foreach ($line in $list) { [void]$notice.AppendLine('  ' + $line) }
foreach ($dir in Get-ChildItem (Join-Path $installed 'share') -Directory) {
  $copyright = Join-Path $dir.FullName 'copyright'
  if (Test-Path $copyright) {
    [void]$notice.AppendLine('')
    [void]$notice.AppendLine('==== ' + $dir.Name + ' ' + ('=' * [Math]::Max(4, 70 - $dir.Name.Length)))
    [void]$notice.AppendLine((Get-Content -Raw -LiteralPath $copyright))
  }
}
Set-Content -Encoding utf8 -Path (Join-Path $Out 'THIRD-PARTY-LICENSES.txt') -Value $notice.ToString()

# It runs, and has the three languages.
$env:TESSDATA_PREFIX = $data
Invoke-Native 'tesseract --version' { & (Join-Path $Out 'tesseract.exe') --version }
$langs = (& (Join-Path $Out 'tesseract.exe') --list-langs 2>&1) -join "`n"
foreach ($lang in $Languages.Keys) {
  if ($langs -notmatch "(?m)^$lang\s*$") { throw "tesseract --list-langs doesn't list $lang`n$langs" }
}
# And it reads Sinhala.
$line = Join-Path $PSScriptRoot 'sample\sinhala-line.png'
if (Test-Path $line) {
  $read = (& (Join-Path $Out 'tesseract.exe') $line stdout -l sin+eng 2>&1) -join "`n"
  Write-Output "Sinhala test read: $read"
  if ([regex]::Matches($read, '[\u0D80-\u0DFF]').Count -lt 20) { throw "tesseract read no Sinhala from $line`n$read" }
}
Get-ChildItem -Recurse $Out | Format-Table FullName, Length
