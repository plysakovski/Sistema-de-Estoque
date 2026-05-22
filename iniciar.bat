@echo off
title BD Fields Osorio
color 0A

echo.
echo  ============================================
echo   BD Fields Osorio - Iniciando...
echo  ============================================
echo.

:: ── Verifica Python ──────────────────────────
set PY=
python --version >nul 2>&1
if %errorlevel% equ 0 set PY=python
if "%PY%"=="" (
    python3 --version >nul 2>&1
    if %errorlevel% equ 0 set PY=python3
)
if "%PY%"=="" (
    echo  ERRO: Python nao encontrado.
    echo  Instale em: https://www.python.org/downloads/
    echo  Marque "Add Python to PATH" durante a instalacao.
    echo.
    pause & exit /b 1
)

:: ── Resolve pasta do .bat (funciona com UNC e drive mapeado) ──
:: %~dp0 sempre retorna o caminho real do .bat, inclusive em rede
set BAT_PATH=%~dp0
:: Remove barra final
if "%BAT_PATH:~-1%"=="\" set BAT_PATH=%BAT_PATH:~0,-1%

:: Verifica se e caminho UNC (comeca com \\)
set IS_UNC=0
echo %BAT_PATH% | findstr /B /C:"\\\\" >nul 2>&1
if %errorlevel% equ 0 set IS_UNC=1

if "%IS_UNC%"=="1" (
    :: Caminho UNC - tenta pushd que suporta UNC nativamente e cria drive temp
    pushd "%BAT_PATH%" >nul 2>&1
    if %errorlevel% neq 0 (
        :: pushd falhou - tenta subst como fallback
        subst Z: /d >nul 2>&1
        subst Z: "%BAT_PATH%\" >nul 2>&1
        if %errorlevel% neq 0 (
            echo  ERRO: Nao foi possivel acessar o caminho de rede.
            echo  Caminho: %BAT_PATH%
            echo  Tente mapear a pasta como unidade de rede no Windows Explorer.
            pause & exit /b 1
        )
        Z:
    )
) else (
    :: Caminho local ou drive ja mapeado - cd direto
    cd /d "%BAT_PATH%"
)

:: ── Confirma que server.py existe ──
if not exist "server.py" (
    echo  ERRO: server.py nao encontrado em:
    echo  %CD%
    echo.
    echo  Certifique-se que server.py esta na mesma pasta que este .bat
    if "%IS_UNC%"=="1" (popd >nul 2>&1 & subst Z: /d >nul 2>&1)
    pause & exit /b 1
)

echo  Pasta  : %CD%
echo  Acesse : http://localhost:8000
echo  Mantenha esta janela aberta.
echo  Para encerrar: feche esta janela.
echo  ============================================
echo.

:: ── Abre o navegador apos 2s ──
start "" /min cmd /c "timeout /t 2 >nul && start http://localhost:8000/IT_Inventory.html"

:: ── Inicia servidor (Python usa __file__ para resolver o caminho) ──
%PY% server.py

:: ── Limpeza ao encerrar ──
if "%IS_UNC%"=="1" (popd >nul 2>&1 & subst Z: /d >nul 2>&1)
pause
