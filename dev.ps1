# Windows development script for MagicTunnel (mirrors dev.sh)

[CmdletBinding()]
param(
    [Alias('r')]
    [switch]$Release
)

$ErrorActionPreference = 'Stop'

if ($Release) {
    Write-Host "🚀 MagicTunnel Release Mode Helper"
    Write-Host "=================================="
} else {
    Write-Host "🚀 MagicTunnel Development Helper"
    Write-Host "================================="
}

function Test-EnvConfigured {
    if (-not (Test-Path .env)) { return $false }
    $envContent = Get-Content .env -Raw
    if ($envContent -match 'OPENAI_API_KEY=sk-your-openai-key-here') { return $false }
    if ($envContent -notmatch 'OPENAI_API_KEY=\S') { return $false }
    return $true
}

if (-not (Test-EnvConfigured)) {
    Write-Host "📝 Setting up .env file..."
    if (-not (Test-Path .env)) {
        Write-Host "Creating .env from example..."
        Copy-Item .env.example .env
    }
    Write-Host ""
    Write-Host "⚠️  Please edit .env file and set your OpenAI API key:"
    Write-Host "   OPENAI_API_KEY=sk-your-actual-openai-key-here"
    Write-Host ""
    Write-Host "Then run this script again:"
    Write-Host "  .\dev.ps1            # Development mode"
    Write-Host "  .\dev.ps1 -Release   # Release mode"
    exit 1
}

Write-Host "✅ .env file configured"

function Test-NeedsRebuild {
    param([string]$BinaryPath)
    if (-not (Test-Path $BinaryPath)) { return $true }
    $srcMtime = (Get-Item src\main.rs).LastWriteTime
    $binMtime = (Get-Item $BinaryPath).LastWriteTime
    return $srcMtime -gt $binMtime
}

if ($Release) {
    if (Test-NeedsRebuild 'target\release\magictunnel.exe') {
        Write-Host "🔨 Building project in release mode..."
        cargo build --release
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    Write-Host "🚀 Starting MagicTunnel (Release) with Smart Discovery..."
    Write-Host "   - OpenAI API configured"
    Write-Host "   - Release mode optimizations enabled"
    Write-Host "   - Info logging enabled"
    Write-Host "   - Server will start on port 3001"
    Write-Host ""
    cargo run --bin magictunnel --release -- --config magictunnel-config.yaml --log-level info
} else {
    if (Test-NeedsRebuild 'target\debug\magictunnel.exe') {
        Write-Host "🔨 Building project..."
        cargo build
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    Write-Host "🚀 Starting MagicTunnel with Smart Discovery..."
    Write-Host "   - OpenAI API configured"
    Write-Host "   - Debug logging enabled"
    Write-Host "   - Server will start on port 3001"
    Write-Host ""
    cargo run --bin magictunnel -- --config magictunnel-config.yaml --log-level debug
}
