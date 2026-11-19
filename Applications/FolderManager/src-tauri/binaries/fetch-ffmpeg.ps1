# Downloads the LGPL ffmpeg build the Windows bundle carries as its sidecar, checked against the release's published checksum.
$ErrorActionPreference = "Stop"
$name = "ffmpeg-n9.0-latest-win64-lgpl-9.0"
$base = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest"
$here = $PSScriptRoot
$zip = Join-Path $env:RUNNER_TEMP "$name.zip"
Invoke-WebRequest "$base/$name.zip" -OutFile $zip
# GitHub serves release assets as octet-stream, so the checksums are saved and read back as text.
$sumsFile = Join-Path $env:RUNNER_TEMP "checksums.sha256"
Invoke-WebRequest "$base/checksums.sha256" -OutFile $sumsFile
$expected = (Get-Content $sumsFile | Where-Object { $_ -match [regex]::Escape("$name.zip") } | Select-Object -First 1) -replace "\s.*$", ""
$actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
if (-not $expected -or $actual -ne $expected.ToLower()) { throw "ffmpeg checksum mismatch: got $actual, expected $expected" }
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [System.IO.Compression.ZipFile]::OpenRead($zip)
try {
  $entry = $archive.Entries | Where-Object { $_.FullName -eq "$name/bin/ffmpeg.exe" }
  [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $here "ffmpeg-x86_64-pc-windows-msvc.exe"), $true)
} finally {
  $archive.Dispose()
}
