[CmdletBinding()]
param(
    [switch]$Native,
    [switch]$PullRuntime
)

$ErrorActionPreference = "Stop"

function Require-Command([string]$Name, [string]$InstallHint) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name is required. $InstallHint"
    }
}

Set-Location (Resolve-Path (Join-Path $PSScriptRoot ".."))

Require-Command "git" "Install Git for Windows."
Require-Command "git-lfs" "Install Git LFS for Windows."
Require-Command "bun" "Install Bun (for example: winget install --id Oven-sh.Bun --exact --scope user). Restart PowerShell after installation."

if ($PullRuntime) {
    git lfs pull --include "assets/runtime/**,assets/third-party/**"
}

bun run setup

if ($Native) {
    Require-Command "rustup" "Install rustup with https://rustup.rs, then open a new PowerShell."
    Require-Command "cargo" "Install rustup with https://rustup.rs, then open a new PowerShell."
    rustup target add x86_64-pc-windows-msvc
    cargo build -p native-client --release --target x86_64-pc-windows-msvc
}

Write-Host "Browser setup is ready. Start it with: bun run dev"
if ($Native) {
    Write-Host "Native binary: target/x86_64-pc-windows-msvc/release/native-client.exe"
}
