@echo off
rem Builds hud_sim.exe (LVGL + ship_hud.ino on the host) with MSVC. Run from anywhere.
setlocal
set ROOT=%~dp0..\..
set LVGL=%ROOT%\.pio\libdeps\esp32\lvgl
if not exist "%LVGL%\lvgl.h" (echo LVGL not found at %LVGL% - run "pio pkg install" first & exit /b 1)
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
set OUT=%~dp0build
if not exist "%OUT%" mkdir "%OUT%"
dir /s /b "%LVGL%\src\*.c" > "%OUT%\lvgl_sources.txt"
set FLAGS=/nologo /O2 /w /MP /DLV_CONF_INCLUDE_SIMPLE /I"%ROOT%\include" /I"%LVGL%" /I"%~dp0stubs" /I"%ROOT%\ship_hud"
pushd "%OUT%"
if not exist lvgl.lib (
  cl %FLAGS% /c @lvgl_sources.txt || (popd & exit /b 1)
  lib /nologo /out:lvgl.lib *.obj || (popd & exit /b 1)
  del *.obj
)
cl %FLAGS% /std:c++17 /EHsc /Tp"%~dp0sim_main.cpp" lvgl.lib /Fe:hud_sim.exe || (popd & exit /b 1)
popd
echo built %OUT%\hud_sim.exe
