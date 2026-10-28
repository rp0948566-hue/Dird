$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Stage = Join-Path $Root 'dist/RetinaCare'
$Cache = if ($env:RETINACARE_CACHE) { $env:RETINACARE_CACHE } else { Join-Path $env:TEMP 'retinacare-runtimes' }
$Runtime = Join-Path $Cache 'python-windows.tar.gz'
$RuntimeUrl = 'https://github.com/astral-sh/python-build-standalone/releases/download/20261021/cpython-3.13.17%2B20261021-x86_64-pc-windows-msvc-install_only_stripped.tar.gz'
$RuntimeSha = '56cde3b1a1dcc36576753cb408ee0c3568d211baafa0eee40423adc147c480f2'

New-Item -ItemType Directory -Force $Cache | Out-Null
if (!(Test-Path $Runtime)) { Invoke-WebRequest -Uri $RuntimeUrl -OutFile $Runtime }
if ((Get-FileHash $Runtime -Algorithm SHA256).Hash.ToLowerInvariant() -ne $RuntimeSha) { throw 'Python runtime checksum mismatch' }

cargo build --release --manifest-path (Join-Path $Root 'native/Cargo.toml') -p retinacare-ui
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force $Stage, (Join-Path $Stage 'resources/ai_engine'), (Join-Path $Stage 'resources/models') | Out-Null
Copy-Item (Join-Path $Root 'native/target/release/RetinaCare.exe') $Stage
tar -xzf $Runtime -C $Stage
Move-Item (Join-Path $Stage 'python') (Join-Path $Stage 'runtime')
& (Join-Path $Stage 'runtime/python.exe') -m pip install --disable-pip-version-check --no-cache-dir --only-binary=:all: --no-deps Pillow==13.0.0
Copy-Item -Recurse (Join-Path $Root 'ai_engine/src') (Join-Path $Stage 'resources/ai_engine/src')
Copy-Item (Join-Path $Root 'models/README.md'), (Join-Path $Root 'models/manifest.example.json') (Join-Path $Stage 'resources/models')
Copy-Item (Join-Path $Root 'LICENSE'), (Join-Path $Root 'README.md') $Stage
Copy-Item (Join-Path $Root 'scripts/test-windows.ps1') $Stage

& (Join-Path $Stage 'RetinaCare.exe') --smoke-test (Join-Path $Stage 'smoke-test.json')
if ($LASTEXITCODE -ne 0) { throw 'Packaged application smoke test failed' }
Get-FileHash (Join-Path $Stage 'RetinaCare.exe') -Algorithm SHA256 | ForEach-Object { "$($_.Hash.ToLowerInvariant())  RetinaCare.exe" } | Set-Content (Join-Path $Stage 'SHA256SUMS')
Compress-Archive -Path $Stage -DestinationPath (Join-Path $Root 'dist/RetinaCare-Windows-x86_64.zip') -Force
Get-FileHash (Join-Path $Root 'dist/RetinaCare-Windows-x86_64.zip') -Algorithm SHA256 | ForEach-Object { "$($_.Hash.ToLowerInvariant())  RetinaCare-Windows-x86_64.zip" } | Set-Content (Join-Path $Root 'dist/RetinaCare-Windows-x86_64.zip.sha256')
Write-Host "Built $Stage"
