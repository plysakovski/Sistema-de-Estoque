@echo off
pushd "%~dp0"
echo Iniciando StockManager Pro...
echo Acesse: http://localhost:8000
start "" "http://localhost:8000/StockManager_Pro.html"
python server_commercial.py
popd
