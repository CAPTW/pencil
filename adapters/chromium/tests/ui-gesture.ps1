# Task-owned browser UI gesture helper for the installed-package smoke test.
# It finds a browser UI element by its accessible name through Windows UI
# Automation (no screenshot, OCR or key logging) and clicks it with real OS
# mouse input (SetCursorPos + mouse_event), after checking that the click
# point hits that element. -Method Invoke uses the element's UI Automation
# Invoke pattern instead. Only windows of the browser process that owns the
# window titled -WindowTitle are searched. Output is one JSON object.
param(
  [Parameter(Mandatory)][ValidateSet('Click', 'Exists')][string]$Command,
  [Parameter(Mandatory)][string]$WindowTitle,
  [Parameter(Mandatory)][string]$Name,
  [ValidateSet('Mouse', 'Invoke')][string]$Method = 'Mouse',
  [ValidateSet('Exact', 'Prefix')][string]$Match = 'Exact',
  [int]$TimeoutMs = 10000
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName WindowsBase
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class GestureInput
{
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
    [DllImport("user32.dll")] private static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] private static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, IntPtr pid);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint attach, uint to, bool on);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();

    public static bool Foreground(IntPtr hwnd)
    {
        uint current = GetCurrentThreadId();
        uint foreground = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
        bool attached = foreground != 0 && foreground != current && AttachThreadInput(current, foreground, true);
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        if (attached) AttachThreadInput(current, foreground, false);
        return GetForegroundWindow() == hwnd;
    }

    // Top-level window under a screen point (GA_ROOT).
    public static IntPtr RootAt(int x, int y)
    {
        POINT point = new POINT();
        point.X = x;
        point.Y = y;
        return GetAncestor(WindowFromPoint(point), 2);
    }

    public static void LeftClick(int x, int y)
    {
        SetCursorPos(x, y);
        System.Threading.Thread.Sleep(60);
        mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(40);
        mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
    }
}
'@
[void][GestureInput]::SetProcessDPIAware()
$ae = [Windows.Automation.AutomationElement]
$scope = [Windows.Automation.TreeScope]

function Get-BrowserWindows {
  $all = $ae::RootElement.FindAll($scope::Children, [Windows.Automation.Condition]::TrueCondition)
  $main = $null
  foreach ($window in $all) { if ($window.Current.Name.StartsWith($WindowTitle, [StringComparison]::Ordinal)) { $main = $window; break } }
  if (-not $main) { return $null }
  $owner = $main.Current.ProcessId
  # The main window first, then its bubbles (menus and extension popups).
  $windows = @($main) + @($all | Where-Object { $_.Current.ProcessId -eq $owner -and -not [Windows.Automation.Automation]::Compare($_, $main) })
  return [pscustomobject]@{ Main = $main; Windows = $windows; ProcessId = $owner }
}

function Find-Named($browser) {
  $condition = if ($Match -eq 'Exact') { [Windows.Automation.PropertyCondition]::new($ae::NameProperty, $Name) } else { [Windows.Automation.Condition]::TrueCondition }
  foreach ($window in $browser.Windows) {
    foreach ($candidate in $window.FindAll($scope::Descendants, $condition)) {
      if ($Match -eq 'Prefix' -and -not $candidate.Current.Name.StartsWith($Name, [StringComparison]::Ordinal)) { continue }
      $rect = $candidate.Current.BoundingRectangle
      if (-not $candidate.Current.IsOffscreen -and -not $rect.IsEmpty -and $rect.Width -gt 0 -and $rect.Height -gt 0) {
        return [pscustomobject]@{ Element = $candidate; Window = $window }
      }
    }
  }
  return $null
}

function Test-Hit($element, $window, [int]$x, [int]$y) {
  # The element under the click point must be the target (or inside it); when
  # hit testing stops at a container, no other top-level window may cover it.
  $hit = $ae::FromPoint([Windows.Point]::new($x, $y))
  $walker = [Windows.Automation.TreeWalker]::ControlViewWalker
  for ($depth = 0; $hit -and $depth -lt 12; $depth++) {
    if ([Windows.Automation.Automation]::Compare($hit, $element)) { return 'element' }
    $hit = $walker.GetParent($hit)
  }
  if ([GestureInput]::RootAt($x, $y) -eq [IntPtr]$window.Current.NativeWindowHandle) { return 'window' }
  return $null
}

# Browser UI labels only (buttons, menu items, windows); never page text or values.
function Get-Diagnostics($browser) {
  if (-not $browser) {
    return @($ae::RootElement.FindAll($scope::Children, [Windows.Automation.Condition]::TrueCondition) |
      ForEach-Object { $_.Current.ClassName } | Select-Object -Unique -First 20)
  }
  $types = @([Windows.Automation.ControlType]::Button, [Windows.Automation.ControlType]::MenuItem, [Windows.Automation.ControlType]::SplitButton)
  $labels = foreach ($window in $browser.Windows) {
    foreach ($type in $types) {
      $window.FindAll($scope::Descendants, [Windows.Automation.PropertyCondition]::new($ae::ControlTypeProperty, $type)) |
        ForEach-Object { $_.Current.Name } | Where-Object { $_ }
    }
  }
  return @($labels | Select-Object -Unique -First 60)
}

$deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
$browser = $null; $found = $null
do {
  $browser = Get-BrowserWindows
  if ($browser) { $found = Find-Named $browser }
  if (-not $found) { Start-Sleep -Milliseconds 250 }
} while (-not $found -and [DateTime]::UtcNow -lt $deadline)

if (-not $found) {
  [ordered]@{ status = if ($browser) { 'not_found' } else { 'window_not_found' }; name = $Name; labels = Get-Diagnostics $browser } | ConvertTo-Json -Compress
  exit 2
}
$element = $found.Element
$current = $element.Current
$result = [ordered]@{ status = 'found'; name = $Name; controlType = $current.ControlType.ProgrammaticName; method = $Method }
if ($Command -eq 'Exists') { $result | ConvertTo-Json -Compress; exit 0 }

if ($Method -eq 'Invoke') {
  $pattern = $null
  if (-not $element.TryGetCurrentPattern([Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
    $result.status = 'no_invoke_pattern'; $result | ConvertTo-Json -Compress; exit 3
  }
  $pattern.Invoke()
  $result.status = 'clicked'; $result | ConvertTo-Json -Compress; exit 0
}

# Mouse: bring the owning top-level window forward when it is the main window
# (menus and popups are already above it), let any opening animation settle,
# then click the element's centre.
if ([Windows.Automation.Automation]::Compare($found.Window, $browser.Main)) {
  $result.foreground = [GestureInput]::Foreground([IntPtr]$browser.Main.Current.NativeWindowHandle)
}
Start-Sleep -Milliseconds 300
$rect = $element.Current.BoundingRectangle
$x = [int]($rect.X + $rect.Width / 2); $y = [int]($rect.Y + $rect.Height / 2)
$result.point = @($x, $y)
$result.hit = Test-Hit $element $found.Window $x $y
if (-not $result.hit) { $result.status = 'occluded'; $result | ConvertTo-Json -Compress; exit 4 }
[GestureInput]::LeftClick($x, $y)
$result.status = 'clicked'
$result | ConvertTo-Json -Compress
exit 0
