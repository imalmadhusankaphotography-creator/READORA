<#
.SYNOPSIS
  Start the packaged readora.exe the way a user does, open a PDF, and take screenshots.

.DESCRIPTION
  Unzips the portable package into a temp folder, starts readora.exe with the control channel
  (--control FILE) and the sample PDF (packaging/readora/sample/readora-sample.pdf), and waits
  until the running app answers. Then it takes screenshots through readora-cli ui (the document,
  the About window, Home) into -Shots, and quits the app. It reports what happened as annotations
  on the GitHub Actions run: the outcome, the process state and the app's own log
  (READORAData\logs), so a start-up failure on Windows can be read without the machine.
  Exits 1 when the window never came up.

.EXAMPLE
  pwsh packaging/readora/smoke-test.ps1 -Zip dist\readora-0.21.1-windows-x64-portable.zip
#>
param(
  [Parameter(Mandatory)] [string] $Zip,
  [string] $Shots = 'screenshots',
  [int] $Seconds = 150
)

# One annotation per chunk: GitHub drops a message that is too long, and keeps at most 10 per step.
function Write-Annotation([string] $Level, [string] $Text) {
  $chunk = 3000
  $parts = [Math]::Min(8, [Math]::Ceiling($Text.Length / $chunk))
  if ($parts -lt 1) { $parts = 1 }
  for ($i = 0; $i -lt $parts; $i++) {
    $start = $i * $chunk
    $part = if ($start -lt $Text.Length) { $Text.Substring($start, [Math]::Min($chunk, $Text.Length - $start)) } else { '' }
    $esc = $part.Replace('%', '%25').Replace("`r", '').Replace("`n", '%0A').Replace('::', ': :')
    Write-Output "::$Level title=Smoke test READORA ($($i + 1)/$parts)::$esc"
  }
}

$report = New-Object System.Collections.Generic.List[string]
$ok = $false
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
New-Item -ItemType Directory -Force -Path $Shots | Out-Null
$Shots = (Resolve-Path $Shots).Path
try {
  $dir = Join-Path ([IO.Path]::GetTempPath()) 'smoke-readora'
  Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
  Expand-Archive -Path $Zip -DestinationPath $dir -Force
  $exe = Get-ChildItem -Path $dir -Recurse -Filter 'readora.exe' | Select-Object -First 1
  if (-not $exe) { throw "readora.exe is not in $Zip" }
  $cli = Join-Path $exe.DirectoryName 'readora-cli.exe'
  $data = Join-Path $exe.DirectoryName 'READORAData'
  $sample = Join-Path $Root 'packaging\readora\sample\readora-sample.pdf'
  $report.Add("Executable: $($exe.FullName) ($([int]($exe.Length / 1MB)) MB)")
  $report.Add("Portable marker: $(Test-Path (Join-Path $exe.DirectoryName 'portable.txt')); OCR models: $(@(Get-ChildItem (Join-Path $exe.DirectoryName 'models') -ErrorAction SilentlyContinue).Count) files")

  # The control file must be in a folder only this user can write.
  $control = Join-Path $env:USERPROFILE '.readora-smoke-control.json'
  Remove-Item -Force $control -ErrorAction SilentlyContinue
  $env:RUST_BACKTRACE = '1'
  $p = Start-Process -FilePath $exe.FullName -ArgumentList '--control', "`"$control`"", "`"$sample`"" -WorkingDirectory $exe.DirectoryName -PassThru
  $null = $p.Handle  # keeps the handle, so ExitCode is readable after the process exits
  $reply = $null
  $started = Get-Date
  while (((Get-Date) - $started).TotalSeconds -lt $Seconds) {
    Start-Sleep -Milliseconds 1000
    if ($p.HasExited) { break }
    if (-not (Test-Path $control)) { continue }
    $out = & $cli ui --control $control state 2>&1
    if ($LASTEXITCODE -eq 0) { $reply = ($out -join "`n"); break }
  }
  $elapsed = [int]((Get-Date) - $started).TotalSeconds
  $ok = [bool]$reply
  if ($ok) {
    function Invoke-Ui([string[]] $UiArgs) {
      $o = & $cli ui --control $control @UiArgs 2>&1
      $report.Add("ui $($UiArgs -join ' '): exit $LASTEXITCODE $((($o -join ' ') -replace '\s+', ' ').Substring(0, [Math]::Min(160, (($o -join ' ') -replace '\s+', ' ').Length)))")
    }
    Invoke-Ui @('resize', 'width=1366', 'height=768')
    Start-Sleep -Seconds 4
    Invoke-Ui @('screenshot', '--out', (Join-Path $Shots 'readora-document.png'))
    Invoke-Ui @('command', 'id=view.layout.two_up')
    Start-Sleep -Seconds 3
    Invoke-Ui @('screenshot', '--out', (Join-Path $Shots 'readora-two-pages.png'))
    Invoke-Ui @('command', 'id=help.about')
    Start-Sleep -Seconds 2
    Invoke-Ui @('screenshot', '--out', (Join-Path $Shots 'readora-about.png'))
    Invoke-Ui @('key', 'key=Escape')
    Invoke-Ui @('click', 'label=Home')
    Start-Sleep -Seconds 2
    Invoke-Ui @('screenshot', '--out', (Join-Path $Shots 'readora-home.png'))
    Invoke-Ui @('quit')
    Start-Sleep -Seconds 3
  }
  $alive = -not $p.HasExited
  $state = if ($alive) { 'still running' } else { "exited, code $($p.ExitCode)" }
  if ($alive) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue; Start-Sleep -Seconds 2 }
  $report.Insert(0, "Result: $(if ($ok) { 'the window came up and answered' } else { 'the window did NOT come up' }) after $elapsed s; process $state")
  $report.Add("State: $(if ($reply) { $reply.Substring(0, [Math]::Min(600, $reply.Length)) } else { '(none)' })")
  $report.Add("Screenshots: $((Get-ChildItem $Shots -Filter *.png | ForEach-Object { "$($_.Name) $([int]($_.Length / 1KB)) KB" }) -join ', ')")
  foreach ($log in Get-ChildItem -Path $data -Recurse -Filter '*.log' -ErrorAction SilentlyContinue) {
    $report.Add("==== $($log.Name)")
    foreach ($line in (Get-Content -LiteralPath $log.FullName -Tail 40 -ErrorAction SilentlyContinue)) { $report.Add($line) }
  }
} catch {
  $report.Insert(0, "Result: the smoke test itself failed: $($_.Exception.Message)")
  $report.Add("$($_.ScriptStackTrace)")
}
$text = ($report -join "`n")
Write-Output $text
Write-Annotation $(if ($ok) { 'notice' } else { 'error' }) $text
if (-not $ok) { exit 1 }
