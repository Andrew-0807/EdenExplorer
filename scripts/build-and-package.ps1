param(
  # Also copy Axis.exe to %LOCALAPPDATA%\Programs\Axis.
  [switch]$Install
)

$ErrorActionPreference = "Stop"

Write-Host "🔨 Building Axis..."

cargo build --release --verbose

$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "target" }
$exe = Join-Path $targetDir "release/Axis.exe"

mkdir -Force release

if (Test-Path $exe) {
  Copy-Item $exe "release/Axis-portable.exe"
} else {
  Write-Error "❌ No binary found at $exe"
}

if (Test-Path "README.md") { Copy-Item "README.md" "release/" }
if (Test-Path "LICENSE") { Copy-Item "LICENSE" "release/" }

if ($Install) {
  $installDir = Join-Path $env:LOCALAPPDATA "Programs\Axis"
  New-Item -ItemType Directory -Force $installDir | Out-Null
  Copy-Item $exe (Join-Path $installDir "Axis.exe") -Force
  Write-Host "📦 Installed to $installDir"
}

Write-Host "✅ Build + packaging complete"
