# Task-owned browser UI gesture helper for the installed-package smoke test.
# It finds browser UI elements by accessible name through Windows UI
# Automation (no screenshot, OCR or key logging) and clicks them with real OS
# mouse input (SetCursorPos + mouse_event), after checking that the click
# point hits the element. -Method Invoke uses the element's UI Automation
# Invoke pattern instead. Only windows of the browser process that owns the
# window titled -WindowTitle are searched. Output is one JSON object.
#   Click / Exists: one element named -Name.
#   Sequence: -Name holds several names separated by '|' ('prefix:' marks a
#   prefix match), clicked in order within this one process, so a menu or
#   popup opened by one click is still open for the next. A step is skipped
#   when the next target is already shown (its menu is open). A failed step
#   adds a content-free timeline (foreground window class, browser window count).
param(
  [Parameter(Mandatory)][ValidateSet('Click', 'Exists', 'Sequence')][string]$Command,
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
using System.Text;
public static class GestureInput
{
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
    [DllImport("user32.dll")] private static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] private static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetClassNameW(IntPtr hwnd, StringBuilder name, int count);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint attach, uint to, bool on);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();

    public static bool Foreground(IntPtr hwnd)
    {
        uint current = GetCurrentThreadId();
        uint ignored;
        uint foreground = GetWindowThreadProcessId(GetForegroundWindow(), out ignored);
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

    // Class name of the foreground window, marked with whether it belongs to
    // process `owner` (content-free: no titles).
    public static string ForegroundClass(uint owner)
    {
        IntPtr hwnd = GetForegroundWindow();
        if (hwnd == IntPtr.Zero) return "none";
        uint pid;
        GetWindowThreadProcessId(hwnd, out pid);
        StringBuilder name = new StringBuilder(128);
        GetClassNameW(hwnd, name, name.Capacity);
        return (pid == owner ? "browser:" : "other:") + name.ToString();
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

function Test-Rect($rect) {
  return -not $rect.IsEmpty -and $rect.Width -gt 0 -and $rect.Height -gt 0 -and
    -not [double]::IsNaN($rect.X) -and -not [double]::IsInfinity($rect.X) -and
    -not [double]::IsNaN($rect.Y) -and -not [double]::IsInfinity($rect.Y)
}

function Find-Named($browser, [string]$target, [string]$match) {
  $condition = if ($match -eq 'Exact') { [Windows.Automation.PropertyCondition]::new($ae::NameProperty, $target) } else { [Windows.Automation.Condition]::TrueCondition }
  foreach ($window in $browser.Windows) {
    foreach ($candidate in $window.FindAll($scope::Descendants, $condition)) {
      if ($match -eq 'Prefix' -and -not ([string]$candidate.Current.Name).StartsWith($target, [StringComparison]::Ordinal)) { continue }
      if (-not $candidate.Current.IsOffscreen -and (Test-Rect $candidate.Current.BoundingRectangle)) {
        return [pscustomobject]@{ Element = $candidate; Window = $window }
      }
    }
  }
  return $null
}

function Test-Hit($element, $window, [int]$x, [int]$y) {
  # The element under the click point must be the target or inside it. If hit
  # testing stops at a container of the target instead, that container must be
  # in the target's own top-level window (nothing else covers the point).
  $hit = $ae::FromPoint([Windows.Point]::new($x, $y))
  $walker = [Windows.Automation.TreeWalker]::ControlViewWalker
  $node = $hit
  for ($depth = 0; $node -and $depth -lt 12; $depth++) {
    if ([Windows.Automation.Automation]::Compare($node, $element)) { return 'element' }
    $node = $walker.GetParent($node)
  }
  $node = $walker.GetParent($element)
  for ($depth = 0; $node -and $hit -and $depth -lt 12; $depth++) {
    if ([Windows.Automation.Automation]::Compare($node, $hit)) {
      if ([GestureInput]::RootAt($x, $y) -eq [IntPtr]$window.Current.NativeWindowHandle) { return 'container' }
      return $null
    }
    $node = $walker.GetParent($node)
  }
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

# Foreground window class and browser window count, sampled for about 2 s.
function Get-Timeline($owner) {
  $samples = @()
  for ($index = 0; $index -lt 10; $index++) {
    $count = -1
    try { $browser = Get-BrowserWindows; if ($browser) { $count = @($browser.Windows).Count } } catch { }
    $samples += '{0}ms {1} windows={2}' -f ($index * 200), [GestureInput]::ForegroundClass([uint32]$owner), $count
    Start-Sleep -Milliseconds 200
  }
  return $samples
}

# Until the deadline: find the element, then invoke it or, for Mouse, click its
# centre after checking the click point hits it. Menus re-create their items
# while they open, so a vanished element, an empty rectangle or a covered point
# is looked up again rather than clicked. Returns a result; never exits.
function Invoke-Target([string]$target, [string]$match, [string]$command, [int]$timeoutMs) {
  $deadline = [DateTime]::UtcNow.AddMilliseconds($timeoutMs)
  $browser = $null
  $last = [ordered]@{ status = 'window_not_found'; name = $target; method = $Method }
  $settled = $false
  do {
    try {
      $browser = Get-BrowserWindows
      $found = if ($browser) { Find-Named $browser $target $match } else { $null }
      if (-not $found) {
        $last = [ordered]@{ status = if ($browser) { 'not_found' } else { 'window_not_found' }; name = $target; method = $Method }
      } else {
        $element = $found.Element
        $result = [ordered]@{ status = 'found'; name = $target; controlType = $element.Current.ControlType.ProgrammaticName; method = $Method }
        if ($command -eq 'Exists') { return $result }
        if ($Method -eq 'Invoke') {
          $pattern = $null
          if (-not $element.TryGetCurrentPattern([Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
            $result.status = 'no_invoke_pattern'; return $result
          }
          $pattern.Invoke()
          $result.status = 'clicked'; return $result
        }
        # Mouse: bring the main window forward (menus and popups are already
        # above it) and let an opening animation settle once.
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
            $result.status = 'clicked'; return $result
          }
          $result.status = 'occluded'; $last = $result
        }
      }
    } catch [Windows.Automation.ElementNotAvailableException] {
      $last = [ordered]@{ status = 'stale'; name = $target; method = $Method }
    }
    Start-Sleep -Milliseconds 250
  } while ([DateTime]::UtcNow -lt $deadline)
  if ($last.status -in @('not_found', 'window_not_found')) { $last.labels = Get-Diagnostics $browser }
  return $last
}

function Split-Target([string]$spec) {
  if ($spec.StartsWith('prefix:', [StringComparison]::Ordinal)) { return @($spec.Substring(7), 'Prefix') }
  return @($spec, 'Exact')
}

function Test-Shown([string]$spec) {
  $target, $match = Split-Target $spec
  try {
    $browser = Get-BrowserWindows
    return [bool]($browser -and (Find-Named $browser $target $match))
  } catch [Windows.Automation.ElementNotAvailableException] {
    return $false
  }
}

$codes = @{ found = 0; clicked = 0; not_found = 2; window_not_found = 2; no_invoke_pattern = 3; occluded = 4; stale = 5; no_rect = 6 }
if ($Command -ne 'Sequence') {
  $result = Invoke-Target $Name $Match $Command $TimeoutMs
  $result | ConvertTo-Json -Compress -Depth 4
  exit $codes[[string]$result.status]
}

$specs = @($Name.Split('|'))
$steps = @()
$status = 'clicked'
for ($index = 0; $index -lt $specs.Count; $index++) {
  $target, $match = Split-Target $specs[$index]
  # A menu that is already open needs no click on its button (a click would close it).
  if ($index + 1 -lt $specs.Count -and (Test-Shown $specs[$index + 1])) {
    $steps += [ordered]@{ status = 'skipped_next_shown'; name = $target; method = $Method }
    continue
  }
  $step = Invoke-Target $target $match 'Click' $TimeoutMs
  $steps += $step
  if ($step.status -ne 'clicked') {
    $status = [string]$step.status
    $browser = Get-BrowserWindows
    $owner = if ($browser) { $browser.ProcessId } else { 0 }
    $step.timeline = Get-Timeline $owner
    break
  }
}
[ordered]@{ status = $status; steps = $steps } | ConvertTo-Json -Compress -Depth 5
exit $codes[$status]
