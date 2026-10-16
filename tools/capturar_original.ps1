# Captura a tela do SimAnt original rodando no DOSBox-X, em escala 1:1 (800x600 do Windows 3.1).
# Uso: powershell -File tools\capturar_original.ps1 -Saida tests\referencias\capturas\nome.png [-Espera 28]
# Inicia o DOSBox-X com referencia\dosbox-x.conf, espera, copia a área cliente da janela e encerra.
param([Parameter(Mandatory)][string]$Saida, [int]$Espera = 28, [string]$Cliques = '', [int]$Intervalo = 4, [string]$Teclas = '')
$raiz = Split-Path $PSScriptRoot -Parent
Set-Location $raiz
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public class Jan {
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Ri, B; }
  [StructLayout(LayoutKind.Sequential)] public struct P { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref P p);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int x, int y, uint d, IntPtr e);
}
'@
[Jan]::SetProcessDPIAware() | Out-Null   # coordenadas físicas (tela com escala de DPI)
$exe = Join-Path $raiz 'referencia\dosbox-x\bin\x64\Release\dosbox-x.exe'
$p = Start-Process -FilePath $exe -PassThru -ArgumentList @('-conf', 'referencia\dosbox-x.conf', '-nopromptfolder')
Start-Sleep -Seconds $Espera
$p.Refresh(); $h = $p.MainWindowHandle
[Jan]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x3) | Out-Null   # HWND_TOPMOST, sem mover/redimensionar
[Jan]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 800
$r = New-Object Jan+R; [Jan]::GetClientRect($h, [ref]$r) | Out-Null
$o = New-Object Jan+P; [Jan]::ClientToScreen($h, [ref]$o) | Out-Null
$w = $r.Ri - $r.L; $a = $r.B - $r.T
foreach ($c in ($Cliques -split ';' | Where-Object { $_ })) {
  $xy = $c -split ','
  [Jan]::SetForegroundWindow($h) | Out-Null
  [Jan]::SetCursorPos($o.X + [int]$xy[0], $o.Y + [int]$xy[1]) | Out-Null
  Start-Sleep -Milliseconds 300
  [Jan]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 120; [Jan]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero)
  Start-Sleep -Seconds $Intervalo
}
foreach ($t in ($Teclas -split '\|' | Where-Object { $_ })) {
  [Jan]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 300
  Add-Type -AssemblyName System.Windows.Forms
  [System.Windows.Forms.SendKeys]::SendWait($t)
  Start-Sleep -Seconds $Intervalo
}
[Jan]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 500
$bmp = New-Object System.Drawing.Bitmap $w, $a
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($o.X, $o.Y, 0, 0, $bmp.Size)
$bmp.Save((Join-Path $raiz $Saida), [System.Drawing.Imaging.ImageFormat]::Png)
Stop-Process -Id $p.Id -Force
"capturado ${w}x${a} -> $Saida"
