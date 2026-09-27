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
#   Keyboard: the same targets through real keyboard input. Alt+Shift+T focuses
#   the toolbar; arrow or Tab keys move focus until UI Automation reports the
#   target focused, and only then Space activates it. Focus names are never
#   recorded (they can be page text); only whether each target was reached.
param(
  [Parameter(Mandatory)][ValidateSet('Click', 'Exists', 'Sequence', 'Keyboard')][string]$Command,
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

    [DllImport("user32.dll")] private static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);

    public static void Key(byte vk)
    {
        keybd_event(vk, 0, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(30);
        keybd_event(vk, 0, 2, UIntPtr.Zero);
    }

    public static void Chord(byte[] keys)
    {
        foreach (byte key in keys) keybd_event(key, 0, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(40);
        for (int index = keys.Length - 1; index >= 0; index--) keybd_event(keys[index], 0, 2, UIntPtr.Zero);
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

$buttonLike = [Windows.Automation.OrCondition]::new([Windows.Automation.Condition[]]@(
  [Windows.Automation.PropertyCondition]::new($ae::ControlTypeProperty, [Windows.Automation.ControlType]::Button),
  [Windows.Automation.PropertyCondition]::new($ae::ControlTypeProperty, [Windows.Automation.ControlType]::MenuItem),
  [Windows.Automation.PropertyCondition]::new($ae::ControlTypeProperty, [Windows.Automation.ControlType]::SplitButton),
  [Windows.Automation.PropertyCondition]::new($ae::ControlTypeProperty, [Windows.Automation.ControlType]::ListItem)))

# Menus and popups (the browser's other top-level windows) are searched before
# the main window; a prefix search looks at button-like controls only.
function Find-Named($browser, [string]$target, [string]$match) {
  $condition = if ($match -eq 'Exact') { [Windows.Automation.PropertyCondition]::new($ae::NameProperty, $target) } else { $buttonLike }
  $ordered = @($browser.Windows | Select-Object -Skip 1) + @($browser.Main)
  foreach ($window in $ordered) {
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
  $started = [DateTime]::UtcNow
  $deadline = $started.AddMilliseconds($timeoutMs)
  $browser = $null
  $last = [ordered]@{ status = 'window_not_found'; name = $target; method = $Method }
  $settled = $false
  # Content-free state changes while this step polls: elapsed ms, foreground
  # window class (browser or other) and the browser's top-level window count.
  $events = New-Object System.Collections.ArrayList
  $lastState = ''
  do {
    try {
      $browser = Get-BrowserWindows
      if ($browser -and $events.Count -lt 40) {
        $state = '{0} windows={1}' -f [GestureInput]::ForegroundClass([uint32]$browser.ProcessId), @($browser.Windows).Count
        if ($state -ne $lastState) {
          [void]$events.Add(('{0}ms {1}' -f [int]([DateTime]::UtcNow - $started).TotalMilliseconds, $state))
          $lastState = $state
        }
      }
      $found = if ($browser) { Find-Named $browser $target $match } else { $null }
      if (-not $found) {
        $last = [ordered]@{ status = if ($browser) { 'not_found' } else { 'window_not_found' }; name = $target; method = $Method }
      } else {
        $element = $found.Element
        $result = [ordered]@{ status = 'found'; name = $target; controlType = $element.Current.ControlType.ProgrammaticName; method = $Method }
        if ($command -eq 'Exists') { $result.events = @($events); return $result }
        if ($Method -eq 'Invoke') {
          # Menu buttons expose ExpandCollapse (or Toggle) instead of Invoke.
          $pattern = $null
          try {
            if ($element.TryGetCurrentPattern([Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
              $pattern.Invoke(); $result.pattern = 'Invoke'
            } elseif ($element.TryGetCurrentPattern([Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$pattern)) {
              $pattern.Expand(); $result.pattern = 'ExpandCollapse'
            } elseif ($element.TryGetCurrentPattern([Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) {
              $pattern.Toggle(); $result.pattern = 'Toggle'
            } else {
              $result.status = 'no_invoke_pattern'; $result.events = @($events); return $result
            }
          } catch [System.Runtime.InteropServices.COMException], [System.InvalidOperationException] {
            # The browser refused the accessibility action.
            $result.status = 'invoke_failed'; $result.error = $_.Exception.GetType().Name; $result.events = @($events); return $result
          }
          $result.status = 'clicked'; $result.events = @($events); return $result
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
            $result.status = 'clicked'; $result.events = @($events); return $result
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
  $last.events = @($events)
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

function Test-FocusedName([string]$spec) {
  $target, $match = Split-Target $spec
  try { $name = [string]$ae::FocusedElement.Current.Name } catch { return $false }
  if ($match -eq 'Prefix') { return $name.StartsWith($target, [StringComparison]::Ordinal) }
  return $name -ceq $target
}

# Presses the navigation keys in turn until the target has keyboard focus.
function Move-FocusTo([string]$spec, [byte[]]$keys, [int]$presses) {
  for ($press = 0; $press -le $presses; $press++) {
    if (Test-FocusedName $spec) { return $true }
    [GestureInput]::Key($keys[$press % $keys.Length])
    Start-Sleep -Milliseconds 150
  }
  return (Test-FocusedName $spec)
}

$codes = @{ found = 0; clicked = 0; not_found = 2; window_not_found = 2; no_invoke_pattern = 3; invoke_failed = 3; occluded = 4; stale = 5; no_rect = 6; focus_not_reached = 7 }
if ($Command -eq 'Keyboard') {
  $browser = Get-BrowserWindows
  if (-not $browser) { [ordered]@{ status = 'window_not_found'; steps = @() } | ConvertTo-Json -Compress -Depth 5; exit 2 }
  $focused = [GestureInput]::Foreground([IntPtr]$browser.Main.Current.NativeWindowHandle)
  Start-Sleep -Milliseconds 300
  $specs = @($Name.Split('|'))
  $steps = @()
  $status = 'clicked'
  # Alt+Shift+T focuses the first toolbar item; Right (then Tab) walks the
  # toolbar, Tab walks menus and popups.
  [GestureInput]::Chord([byte[]]@(0x12, 0x10, 0x54))
  Start-Sleep -Milliseconds 400
  for ($index = 0; $index -lt $specs.Count; $index++) {
    $target, $match = Split-Target $specs[$index]
    # A toolbar button already showing the next target (a pinned action) needs no menu.
    if ($index -eq 0 -and $specs.Count -gt 1 -and (Test-Shown $specs[1])) {
      $steps += [ordered]@{ status = 'skipped_next_shown'; name = $target; method = 'Keyboard' }
      continue
    }
    $keys = if ($index -le 1 -and $steps.Count -gt 0 -and $steps[0].status -eq 'skipped_next_shown') { [byte[]]@(0x27) } elseif ($index -eq 0) { [byte[]]@(0x27) } else { [byte[]]@(0x09) }
    $reached = Move-FocusTo $specs[$index] $keys 30
    if (-not $reached -and $keys[0] -eq 0x27) { $reached = Move-FocusTo $specs[$index] ([byte[]]@(0x09)) 30 }
    if (-not $reached) {
      $steps += [ordered]@{ status = 'focus_not_reached'; name = $target; method = 'Keyboard'; foreground = $focused }
      $status = 'focus_not_reached'
      break
    }
    [GestureInput]::Key(0x20)
    $steps += [ordered]@{ status = 'clicked'; name = $target; method = 'Keyboard'; foreground = $focused }
    Start-Sleep -Milliseconds 1000
  }
  [ordered]@{ status = $status; steps = $steps } | ConvertTo-Json -Compress -Depth 5
  exit $codes[$status]
}
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
