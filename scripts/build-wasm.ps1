[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$expectedCliVersion = 'wasm-bindgen 0.2.127'
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$webRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot 'apps\web'))
$outputDirectory = [System.IO.Path]::GetFullPath((Join-Path $webRoot 'src\wasm\generated'))
$manifestPath = Join-Path $repositoryRoot 'Cargo.toml'
$wasmArtifact = Join-Path $repositoryRoot 'target\wasm32-unknown-unknown\release\textscope_wasm.wasm'
$cargoHome = if ([string]::IsNullOrWhiteSpace($env:CARGO_HOME)) {
    Join-Path ([Environment]::GetFolderPath('UserProfile')) '.cargo'
} else {
    $env:CARGO_HOME
}
$cargoHome = [System.IO.Path]::GetFullPath($cargoHome)

if (-not $outputDirectory.StartsWith("$webRoot\", [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to generate bindings outside apps/web: $outputDirectory"
}

$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
if ($null -eq $cargoCommand) {
    throw 'Cargo was not found on PATH. Install Rust with rustup and reopen the terminal.'
}

$wasmBindgenCommand = Get-Command wasm-bindgen -ErrorAction SilentlyContinue
if ($null -eq $wasmBindgenCommand) {
    throw 'wasm-bindgen-cli 0.2.127 is required. Install it with: cargo install wasm-bindgen-cli --version 0.2.127 --locked'
}

$actualCliVersion = (& $wasmBindgenCommand.Source --version 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to execute wasm-bindgen --version.'
}

if ($actualCliVersion -ne $expectedCliVersion) {
    throw "Expected $expectedCliVersion but found $actualCliVersion. Install the exact version with: cargo install wasm-bindgen-cli --version 0.2.127 --locked --force"
}

Write-Output "Using $actualCliVersion"
Write-Output 'Compiling textscope-wasm for wasm32-unknown-unknown...'
$originalEncodedRustFlags = $env:CARGO_ENCODED_RUSTFLAGS
$separator = [char]0x1f
$remapFlags = @(
    "--remap-path-prefix=$repositoryRoot=.",
    "--remap-path-prefix=$cargoHome=.cargo"
)
$env:CARGO_ENCODED_RUSTFLAGS = if ([string]::IsNullOrEmpty($originalEncodedRustFlags)) {
    $remapFlags -join $separator
} else {
    "$originalEncodedRustFlags$separator$($remapFlags -join $separator)"
}

try {
    & $cargoCommand.Source build --manifest-path $manifestPath --package textscope-wasm --target wasm32-unknown-unknown --release
    if ($LASTEXITCODE -ne 0) {
        throw 'Cargo failed to build textscope-wasm for wasm32-unknown-unknown.'
    }
} finally {
    $env:CARGO_ENCODED_RUSTFLAGS = $originalEncodedRustFlags
}

if (-not (Test-Path -LiteralPath $wasmArtifact -PathType Leaf)) {
    throw "Expected WASM artifact was not produced: $wasmArtifact"
}

if (Test-Path -LiteralPath $outputDirectory) {
    Remove-Item -LiteralPath $outputDirectory -Recurse -Force
}
New-Item -ItemType Directory -Path $outputDirectory | Out-Null

Write-Output 'Generating browser bindings...'
& $wasmBindgenCommand.Source $wasmArtifact --target web --out-dir $outputDirectory --out-name textscope_wasm
if ($LASTEXITCODE -ne 0) {
    throw 'wasm-bindgen failed to generate browser bindings.'
}

$expectedFiles = @(
    'textscope_wasm.js',
    'textscope_wasm.d.ts',
    'textscope_wasm_bg.wasm',
    'textscope_wasm_bg.wasm.d.ts'
)
foreach ($fileName in $expectedFiles) {
    $generatedPath = Join-Path $outputDirectory $fileName
    if (-not (Test-Path -LiteralPath $generatedPath -PathType Leaf)) {
        throw "Expected generated binding is missing: $generatedPath"
    }
}

Write-Output "Browser bindings generated in: $outputDirectory"
