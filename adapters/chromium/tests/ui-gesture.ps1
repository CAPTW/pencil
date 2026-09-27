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
  foreach ($window in $all) { if (([string]$window.Current.Name).StartsWith($WindowTitle, [StringComparison]::Ordinal)) { $main = $window; break } }
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
      if ($Match -eq 'Prefix' -and -not ([string]$candidate.Current.Name).StartsWith($Name, [StringComparison]::Ordinal)) { continue }
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

# Button and menu labels only (never page text, values or suggestion labels),
# or top-level window classes when the browser window itself is missing.
function Get-Diagnostics($browser) {
  try {
    if (-not $browser) {
      return @($ae::RootElement.FindAll($scope::Children, [Windows.Automation.Condition]::TrueCondition) |
        ForEach-Object { $_.Current.ClassName } | Select-Object -Unique -First 20)
    }
    $types = @([Windows.Automation.ControlType]::Button, [Windows.Automation.ControlType]::MenuItem, [Windows.Automation.ControlType]::SplitButton)
    $labels = foreach ($window in $browser.Windows) {
      foreach ($type in $types) {
        $window.FindAll($scope::Descendants, [Windows.Automation.PropertyCondition]::new($ae::ControlTypeProperty, $type)) |
          ForEach-Object { [string]$_.Current.Name } | Where-Object { $_ -and -not $_.StartsWith('Suggestion') }
      }
    }
    return @($labels | Select-Object -Unique -First 60)
  } catch {
    return @('diagnostics_unavailable')
  }
}

function Test-Rect($rect) {
  return -not $rect.IsEmpty -and $rect.Width -gt 0 -and $rect.Height -gt 0 -and
    -not [double]::IsNaN($rect.X) -and -not [double]::IsInfinity($rect.X) -and
    -not [double]::IsNaN($rect.Y) -and -not [double]::IsInfinity($rect.Y)
}

function Write-Result($result, [int]$code) {
  $result | ConvertTo-Json -Compress
  exit $code
}

# Until the deadline: find the element, then (Click) invoke it or, for Mouse,
# click its centre after checking the click point hits it. Menus re-create
# their items while they open, so a vanished element, an empty rectangle or a
# covered point is looked up again rather than clicked.
$deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
$browser = $null
$last = [ordered]@{ status = 'window_not_found'; name = $Name; method = $Method }
$settled = $false
do {
  try {
    $browser = Get-BrowserWindows
    $found = if ($browser) { Find-Named $browser } else { $null }
    if (-not $found) {
      $last = [ordered]@{ status = if ($browser) { 'not_found' } else { 'window_not_found' }; name = $Name; method = $Method }
    } else {
      $element = $found.Element
      $result = [ordered]@{ status = 'found'; name = $Name; controlType = $element.Current.ControlType.ProgrammaticName; method = $Method }
      if ($Command -eq 'Exists') { Write-Result $result 0 }
      if ($Method -eq 'Invoke') {
        $pattern = $null
        if (-not $element.TryGetCurrentPattern([Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
          $result.status = 'no_invoke_pattern'; Write-Result $result 3
        }
        $pattern.Invoke()
        $result.status = 'clicked'; Write-Result $result 0
      }
      # Mouse: bring the main window forward (menus and popups are already above
      # it) and let an opening animation settle once before the first click.
      if ([Windows.Automation.Automation]::Compare($found.Window, $browser.Main)) {
        $result.foreground = [GestureInput]::Foreground([IntPtr]$browser.Main.Current.NativeWindowHandle)
      }
      if (-not $settled) { Start-Sleep -Milliseconds 300; $settled = $true }
      $rect = $element.Current.BoundingRectangle
      if (-not (Test-Rect $rect)) {
        $result.status = 'no_rect'; $last = $result
      } else {
        $x = [int]($rect.X + $rect.Width / 2); $y = [int]($rect.Y + $rect.Height / 2)
        $result.point = @($x, $y)
        $result.hit = Test-Hit $element $found.Window $x $y
        if ($result.hit) {
          [GestureInput]::LeftClick($x, $y)
          $result.status = 'clicked'; Write-Result $result 0
        }
        $result.status = 'occluded'; $last = $result
      }
    }
  } catch [Windows.Automation.ElementNotAvailableException] {
    $last = [ordered]@{ status = 'stale'; name = $Name; method = $Method }
  }
  Start-Sleep -Milliseconds 250
} while ([DateTime]::UtcNow -lt $deadline)

if ($last.status -in @('not_found', 'window_not_found')) { $last.labels = Get-Diagnostics $browser }
Write-Result $last $(switch ($last.status) { 'not_found' { 2 } 'window_not_found' { 2 } 'occluded' { 4 } 'stale' { 5 } default { 6 } })
