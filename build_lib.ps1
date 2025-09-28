#!/usr/bin/env pwsh
# build_lib.ps1

# 1. 执行 dotnet publish 命令
Write-Host "正在发布 VpkInfo 项目..." -ForegroundColor Green
dotnet publish ./lib/VpkInfo.csproj -c Release -r win-x64

# 检查发布是否成功
if ($LASTEXITCODE -ne 0) {
    Write-Host "发布失败！退出代码: $LASTEXITCODE" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "发布成功！" -ForegroundColor Green

# 2. 定义源文件和目标路径
$sourceFile = "lib\bin\Release\net9.0\win-x64\publish\vpkinfo.dll"
$destinationDir = "cli\libs\"

# 检查源文件是否存在
if (-not (Test-Path $sourceFile)) {
    Write-Host "错误: 找不到源文件 $sourceFile" -ForegroundColor Red
    Write-Host "请检查发布路径是否正确" -ForegroundColor Yellow
    exit 1
}

# 确保目标目录存在
if (-not (Test-Path $destinationDir)) {
    Write-Host "创建目标目录: $destinationDir" -ForegroundColor Yellow
    New-Item -ItemType Directory -Path $destinationDir -Force | Out-Null
}

# 复制文件
Write-Host "正在复制文件..." -ForegroundColor Green

try {
    Copy-Item -Path $sourceFile -Destination $destinationDir -Force
    Write-Host "文件复制成功！" -ForegroundColor Green
}
catch {
    Write-Host "复制文件时出错: $_" -ForegroundColor Red
    exit 1
}

# 验证复制结果
$copiedFile = Join-Path $destinationDir "vpkinfo.dll"
if (Test-Path $copiedFile) {
    $fileInfo = Get-Item $copiedFile
    Write-Host "验证: 文件已成功复制到 $copiedFile" -ForegroundColor Green
    Write-Host "文件大小: $($fileInfo.Length) 字节" -ForegroundColor Gray
    Write-Host "修改时间: $($fileInfo.LastWriteTime)" -ForegroundColor Gray
} else {
    Write-Host "错误: 文件复制后验证失败" -ForegroundColor Red
    exit 1
}