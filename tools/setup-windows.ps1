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
Require-Command "rustup" "Install rustup with https://rustup.rs, then open a new PowerShell."
Require-Command "cargo" "Install rustup with https://rustup.rs, then open a new PowerShell."
Require-Command "wasm-pack" "Install wasm-pack with: cargo install wasm-pack --version 0.13.1"

rustup target add wasm32-unknown-unknown

if ($PullRuntime) {
    git lfs pull --include "assets/runtime/**,assets/third-party/**"
}

bun run setup

if ($Native) {
    rustup target add x86_64-pc-windows-msvc
    cargo build -p native-client --release --target x86_64-pc-windows-msvc
}

Write-Host "Browser setup is ready. Start it with: bun run dev"
if ($Native) {
    Write-Host "Native binary: target/x86_64-pc-windows-msvc/release/native-client.exe"
}
