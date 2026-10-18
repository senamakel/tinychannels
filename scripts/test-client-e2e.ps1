$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $root
$target = Join-Path $root 'target'
New-Item -ItemType Directory -Force $target | Out-Null
$target = (Resolve-Path -LiteralPath $target).Path
$rootPrefix = $root.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
if (-not $target.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'test output must stay inside the checkout target directory' }

$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$runnerSid = "*$($identity.User.Value):(OI)(CI)F"
$adminSid = '*S-1-5-32-544:(OI)(CI)F'
$systemSid = '*S-1-5-18:(OI)(CI)F'
function Set-PrivateDirectory([string]$Path) {
    New-Item -ItemType Directory -Force $Path | Out-Null
    icacls $Path /reset | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "could not reset test fixture directory permissions: $Path" }
    icacls $Path /inheritance:r /grant:r $runnerSid $adminSid $systemSid | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "could not restrict test fixture directory permissions: $Path" }
    icacls $Path /setowner $identity.Name | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "could not set test fixture directory owner: $Path" }
}

$fixtureManifest = Join-Path $root 'crates/tinychannels-client/test-fixture/Cargo.toml'
cargo build --manifest-path $fixtureManifest --locked --target-dir $target
if ($LASTEXITCODE -ne 0) { throw 'Failed to build tinychannels-client test fixture' }
cargo build --manifest-path (Join-Path $root 'Cargo.toml') --locked --target-dir $target -p tinychannels-module
if ($LASTEXITCODE -ne 0) { throw 'Failed to build the actual TinyChannels module' }

$stage = Join-Path $target 'tinychannels-client-e2e'
$cache = Join-Path $stage 'cache'
$archiveName = 'tinychannels-module-client-test.tar.gz'
$archive = Join-Path $stage $archiveName
$nativeStage = Join-Path $target 'tinychannels-module-client-native-e2e'
$nativeCache = Join-Path $nativeStage 'cache'
$nativeArchiveName = 'tinychannels-module-native-test.tar.gz'
$nativeArchive = Join-Path $nativeStage $nativeArchiveName
Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $nativeStage -Recurse -Force -ErrorAction SilentlyContinue
Set-PrivateDirectory $stage
Set-PrivateDirectory $cache
Set-PrivateDirectory $nativeStage
Set-PrivateDirectory $nativeCache
$libraryName = 'tinychannels_client_test_fixture.dll'
$library = Join-Path $cache $libraryName
Copy-Item -LiteralPath (Join-Path $target "debug/$libraryName") -Destination $library
$nativeLibraryName = 'tinychannels_module.dll'
$nativeLibrary = Join-Path $nativeCache $nativeLibraryName
Copy-Item -LiteralPath (Join-Path $target "debug/$nativeLibraryName") -Destination $nativeLibrary
$libraryHash = (Get-FileHash -LiteralPath $library -Algorithm SHA256).Hash.ToLowerInvariant()
'"{0}" = "{1}"' -f $libraryName, $libraryHash |
    Set-Content -Path (Join-Path $cache 'modules.toml') -Encoding utf8NoBOM
& tar.exe -czf $archive -C $cache $libraryName modules.toml
if ($LASTEXITCODE -ne 0) { throw 'Could not archive the local TinyChannels test fixture' }
Copy-Item -LiteralPath $archive -Destination (Join-Path $cache $archiveName)
$archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
$nativeLibraryHash = (Get-FileHash -LiteralPath $nativeLibrary -Algorithm SHA256).Hash.ToLowerInvariant()
'"{0}" = "{1}"' -f $nativeLibraryName, $nativeLibraryHash |
    Set-Content -Path (Join-Path $nativeCache 'modules.toml') -Encoding utf8NoBOM
& tar.exe -czf $nativeArchive -C $nativeCache $nativeLibraryName modules.toml
if ($LASTEXITCODE -ne 0) { throw 'Could not archive the actual TinyChannels module' }
Copy-Item -LiteralPath $nativeArchive -Destination (Join-Path $nativeCache $nativeArchiveName)
$nativeArchiveHash = (Get-FileHash -LiteralPath $nativeArchive -Algorithm SHA256).Hash.ToLowerInvariant()

$env:TINYCHANNELS_CLIENT_TEST_CACHE = $cache
$env:TINYCHANNELS_CLIENT_TEST_ARCHIVE_NAME = $archiveName
$env:TINYCHANNELS_CLIENT_TEST_ARCHIVE_SHA256 = $archiveHash
$env:TINYCHANNELS_CLIENT_TEST_LIBRARY_SHA256 = $libraryHash
$env:TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_CACHE = $nativeCache
$env:TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_ARCHIVE_NAME = $nativeArchiveName
$env:TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_ARCHIVE_SHA256 = $nativeArchiveHash
$env:TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_LIBRARY_SHA256 = $nativeLibraryHash
$env:TINYCHANNELS_CLIENT_NATIVE_MODULE_TEST_RELEASE_URL = 'https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.37'
cargo test --manifest-path (Join-Path $root 'Cargo.toml') --locked --target-dir $target `
    -p tinychannels-client --features native-e2e --test native_e2e -- --test-threads=1
if ($LASTEXITCODE -ne 0) { throw 'TinyChannels typed client native E2E failed' }
cargo test --manifest-path (Join-Path $root 'Cargo.toml') --locked --target-dir $target `
    -p tinychannels-client --features native-e2e --test runtime_native_e2e -- --test-threads=1
if ($LASTEXITCODE -ne 0) { throw 'TinyChannels runtime driver native E2E failed' }
cargo test --manifest-path (Join-Path $root 'Cargo.toml') --locked --target-dir $target `
    -p tinychannels-client --features native-e2e --test native_module_runtime_e2e -- --test-threads=1
if ($LASTEXITCODE -ne 0) { throw 'Actual TinyChannels module runtime E2E failed' }
