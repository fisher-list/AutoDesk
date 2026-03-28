@echo off
chcp 65001 >nul
echo ================================================
echo   AutoDesk Windows 构建脚本
echo ================================================
echo.

echo [1/5] 检查 Node.js...
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误: 未找到 Node.js，请先安装 https://nodejs.org/
    pause
    exit /b 1
)
node --version
echo.

echo [2/5] 检查 Rust...
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误: 未找到 Rust，请先安装 https://rustup.rs/
    pause
    exit /b 1
)
cargo --version
echo.

echo [3/5] 安装前端依赖...
cd /d "%~dp0client"
call npm install
if %errorlevel% neq 0 (
    echo 错误: npm install 失败
    pause
    exit /b 1
)
echo.

echo [4/5] 安装 Tauri CLI...
cargo install tauri-cli --force
if %errorlevel% neq 0 (
    echo 错误: Tauri CLI 安装失败
    pause
    exit /b 1
)
echo.

echo [5/5] 构建 Windows 安装包...
call npm run tauri build
if %errorlevel% neq 0 (
    echo 错误: 构建失败
    pause
    exit /b 1
)
echo.

echo ================================================
echo   构建完成！
echo ================================================
echo.
echo 安装包位置:
echo   - NSIS: client\src-tauri\target\release\bundle\nsis\AutoDesk_0.1.0_x64-setup.exe
echo   - MSI:  client\src-tauri\target\release\bundle\msi\AutoDesk_0.1.0_x64_en-US.msi
echo.
echo 如需发布到 GitHub:
echo   1. 创建 tag: git tag v0.1.0
echo   2. 推送 tag: git push origin v0.1.0
echo   3. 在 GitHub 创建 Release
echo.
pause
