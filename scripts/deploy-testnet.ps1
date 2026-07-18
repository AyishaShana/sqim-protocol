param(
    [string]$SourceIdentity = $env:SQIM_DEPLOYER_IDENTITY,
    [string]$Network = "testnet"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (-not $SourceIdentity) {
    throw "SQIM_DEPLOYER_IDENTITY or -SourceIdentity is required"
}
if ($Network -ne "testnet") {
    throw "This deployment script is testnet-only"
}
if (-not $env:SQIM_ORACLE_SIGNERS -or -not $env:SQIM_REBALANCERS) {
    throw "SQIM_ORACLE_SIGNERS and SQIM_REBALANCERS are required comma-separated address lists"
}

$repo = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repo "config/testnet.json"
$manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
$admin = (& stellar keys address $SourceIdentity).Trim()
$oracleSigners = @($env:SQIM_ORACLE_SIGNERS.Split(",") | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$rebalancers = @($env:SQIM_REBALANCERS.Split(",") | ForEach-Object { $_.Trim() } | Where-Object { $_ })
if ($oracleSigners.Count -lt 2 -or $rebalancers.Count -lt 2) {
    throw "Sqim testnet deployment requires at least two oracle and two rebalancer addresses"
}

$temp = Join-Path ([IO.Path]::GetTempPath()) ("sqim-deploy-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    Push-Location $repo
    & stellar contract build
    if ($LASTEXITCODE -ne 0) { throw "contract build failed" }

    $release = Join-Path $repo "target/wasm32v1-none/release"
    $basketHash = (& stellar contract upload --network $Network --source $SourceIdentity --wasm (Join-Path $release "basket.wasm")).Trim()
    $tokenHash = (& stellar contract upload --network $Network --source $SourceIdentity --wasm (Join-Path $release "basket_token.wasm")).Trim()
    $oracleHash = (& stellar contract upload --network $Network --source $SourceIdentity --wasm (Join-Path $release "oracle_adapter.wasm")).Trim()
    $settlementHash = (& stellar contract upload --network $Network --source $SourceIdentity --wasm (Join-Path $release "settlement.wasm")).Trim()
    $factoryHash = (& stellar contract upload --network $Network --source $SourceIdentity --wasm (Join-Path $release "factory.wasm")).Trim()

    $oracle = (& stellar contract deploy --network $Network --source $SourceIdentity --wasm-hash $oracleHash).Trim()
    $settlement = (& stellar contract deploy --network $Network --source $SourceIdentity --wasm-hash $settlementHash).Trim()
    $factory = (& stellar contract deploy --network $Network --source $SourceIdentity --wasm-hash $factoryHash).Trim()

    $oracleSignersPath = Join-Path $temp "oracle-signers.json"
    $oracleSigners | ConvertTo-Json | Set-Content -Encoding ascii $oracleSignersPath
    & stellar contract invoke --network $Network --source $SourceIdentity --id $oracle --send=yes -- initialize `
        --admin $admin --primary_oracle $manifest.primaryOracle --primary_enabled true `
        --max_age_seconds 1800 --fallback_signers-file-path $oracleSignersPath --fallback_threshold 2
    if ($LASTEXITCODE -ne 0) { throw "oracle initialization failed" }

    foreach ($symbol in @("XLM", "ETH", "BTC", "SOL")) {
        & stellar contract invoke --network $Network --source $SourceIdentity --id $oracle --send=yes -- set_primary_symbol `
            --admin $admin --asset $manifest.assets.$symbol --symbol $symbol
        if ($LASTEXITCODE -ne 0) { throw "oracle mapping failed for $symbol" }
    }

    & stellar contract invoke --network $Network --source $SourceIdentity --id $settlement --send=yes -- initialize `
        --admin $admin --router $manifest.router --oracle $oracle --max_slippage_bps 200
    if ($LASTEXITCODE -ne 0) { throw "settlement initialization failed" }

    $factoryConfig = [ordered]@{
        admin = $admin
        basket_wasm_hash = $basketHash
        deposit_asset = $manifest.assets.XLM
        max_drift_bps = 1000
        max_transaction_amount = "1000000000"
        oracle = $oracle
        rebalancer_threshold = 2
        rebalancers = $rebalancers
        settlement = $settlement
        token_decimals = 7
        token_wasm_hash = $tokenHash
        withdrawal_fee_bps = 100
    }
    $factoryConfigPath = Join-Path $temp "factory-config.json"
    $factoryConfig | ConvertTo-Json -Depth 4 | Set-Content -Encoding ascii $factoryConfigPath
    & stellar contract invoke --network $Network --source $SourceIdentity --id $factory --send=yes -- initialize `
        --config-file-path $factoryConfigPath
    if ($LASTEXITCODE -ne 0) { throw "factory initialization failed" }

    $assets = @("XLM", "ETH", "BTC", "SOL") | ForEach-Object { @{ address = $manifest.assets.$_ } }
    $assetsPath = Join-Path $temp "assets.json"
    $weightsPath = Join-Path $temp "weights.json"
    $assets | ConvertTo-Json -Depth 3 | Set-Content -Encoding ascii $assetsPath
    @(4000, 2000, 2000, 2000) | ConvertTo-Json | Set-Content -Encoding ascii $weightsPath
    $basket = (& stellar contract invoke --network $Network --source $SourceIdentity --id $factory --send=yes -- create_basket `
        --creator $admin --name "Sqim Core Four" --assets-file-path $assetsPath `
        --target_weights_bps-file-path $weightsPath).Trim().Trim('"')
    if ($LASTEXITCODE -ne 0) { throw "basket creation failed" }

    $spec = (& stellar contract invoke --network $Network --source $SourceIdentity --id $factory --send=no -- basket --id 0) | ConvertFrom-Json
    $manifest.factory = $factory
    $manifest.basket = $basket
    $manifest.basketToken = $spec.basket_token
    $manifest.settlement = $settlement
    $manifest.oracleAdapter = $oracle
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -Encoding ascii $manifestPath

    [ordered]@{
        factory = $factory
        basket = $basket
        basketToken = $spec.basket_token
        settlement = $settlement
        oracleAdapter = $oracle
    } | ConvertTo-Json
}
finally {
    Pop-Location
    if (Test-Path $temp) { Remove-Item -LiteralPath $temp -Recurse -Force }
}
