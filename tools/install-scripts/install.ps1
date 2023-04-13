#!/usr/bin/env pwsh
# Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.
# Forked from Deno's install.ps1 script

$ErrorActionPreference = 'Stop'

if ($v) {
  $Version = "v${v}"
}
if ($Args.Length -eq 1) {
  $Version = $Args.Get(0)
}

$TypstInstall = $env:TYPST_INSTALL
$BinDir = if ($TypstInstall) {
  "${TypstInstall}\bin"
} else {
  "${Home}\.typst\bin"
}

$TypstZip = "$BinDir\typst.zip"
$TypstExe = "$BinDir\typst.exe"
$Target = 'x86_64-pc-windows-msvc'

$DownloadUrl = if (!$Version) {
  "https://github.com/typst/typst/releases/latest/download/typst-${Target}.zip"
} else {
  "https://github.com/typst/typst/releases/download/${Version}/typst-${Target}.zip"
}

if (!(Test-Path $BinDir)) {
  New-Item $BinDir -ItemType Directory | Out-Null
}

curl.exe -Lo $TypstZip $DownloadUrl

tar.exe xf $TypstZip -C $BinDir --strip-components=1

Remove-Item $TypstZip

$User = [System.EnvironmentVariableTarget]::User
$Path = [System.Environment]::GetEnvironmentVariable('Path', $User)
if (!(";${Path};".ToLower() -like "*;${BinDir};*".ToLower())) {
  [System.Environment]::SetEnvironmentVariable('Path', "${Path};${BinDir}", $User)
  $Env:Path += ";${BinDir}"
}

Write-Output "Typst was installed successfully to ${TypstExe}"
Write-Output "Run 'typst --help' to get started"
Write-Output "Stuck? Join our Discord https://discord.gg/2uDybryKPe"
