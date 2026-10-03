param([Parameter(Mandatory)][int]$AppPid)
$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Owned guest only'}
if((Get-Process -Id $AppPid).Path -ine 'C:\GrammarTrial\codex-pencil.exe'){throw 'Unexpected app process'}
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class TrayInput {
 [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x,y; }
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint x,uint y,uint d,UIntPtr e);
 [DllImport("user32.dll")] public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
}
'@
function Element-Owner($item){
 $rect=$item.Current.BoundingRectangle
 if($rect.IsEmpty -or [double]::IsNaN($rect.X)){return -1}
 $point=[TrayInput+POINT]::new();$point.x=[int]($rect.X+$rect.Width/2);$point.y=[int]($rect.Y+$rect.Height/2)
 [uint32]$hitOwner=0;[void][TrayInput]::GetWindowThreadProcessId([TrayInput]::WindowFromPoint($point),[ref]$hitOwner)
 return [int]$hitOwner
}
function Find-Named([string]$name,[int]$owner){
 $condition=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,$name)
 $items=[Windows.Automation.AutomationElement]::RootElement.FindAll([Windows.Automation.TreeScope]::Descendants,$condition)
 foreach($item in $items){if(-not $item.Current.IsOffscreen -and (Element-Owner $item) -eq $owner){return $item}}
 return $null
}
function Click-Verified($item,[int]$owner){
 $rect=$item.Current.BoundingRectangle
 if($rect.IsEmpty -or [double]::IsNaN($rect.X)){throw 'No clickable rectangle'}
 $point=[TrayInput+POINT]::new();$point.x=[int]($rect.X+$rect.Width/2);$point.y=[int]($rect.Y+$rect.Height/2)
 $hit=[TrayInput]::WindowFromPoint($point);[uint32]$hitOwner=0
 [void][TrayInput]::GetWindowThreadProcessId($hit,[ref]$hitOwner)
 if($hitOwner -ne $owner){throw 'Click target owner changed'}
 [void][TrayInput]::SetCursorPos($point.x,$point.y)
 [TrayInput]::mouse_event(2,0,0,0,[UIntPtr]::Zero);[TrayInput]::mouse_event(4,0,0,0,[UIntPtr]::Zero)
}
$shell=@(Get-Process explorer | Where-Object SessionId -eq (Get-Process -Id $PID).SessionId)
if($shell.Count -ne 1){throw 'Ambiguous guest shell'}
$shellPid=$shell[0].Id
$shellElements=[Windows.Automation.AutomationElement]::RootElement.FindAll([Windows.Automation.TreeScope]::Descendants,
 [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ProcessIdProperty,$shellPid))
@($shellElements | ForEach-Object { @{name=$_.Current.Name;automationId=$_.Current.AutomationId;class=$_.Current.ClassName;offscreen=$_.Current.IsOffscreen} }) |
 ConvertTo-Json -Depth 3 | Set-Content C:\GrammarOutput\tray-shell-diagnostic.json -Encoding utf8
$icon=Find-Named 'Codex Pencil' $shellPid
if(-not $icon){
 foreach($label in @('Show hidden icons','숨겨진 아이콘 표시')){
  $overflow=Find-Named $label $shellPid
  if($overflow){Click-Verified $overflow $shellPid;Start-Sleep -Milliseconds 300;break}
 }
 $icon=Find-Named 'Codex Pencil' $shellPid
}
if(-not $icon){
 # Focus the notification area, then move only within the verified guest shell.
 [TrayInput]::keybd_event(0x5B,0,0,[UIntPtr]::Zero);[TrayInput]::keybd_event(0x42,0,0,[UIntPtr]::Zero)
 [TrayInput]::keybd_event(0x42,0,2,[UIntPtr]::Zero);[TrayInput]::keybd_event(0x5B,0,2,[UIntPtr]::Zero)
 Start-Sleep -Milliseconds 300
 $focusLog=@()
 for($i=0;$i -lt 20;$i++){
  $focus=[Windows.Automation.AutomationElement]::FocusedElement
  if(-not $focus){break}
  $owner=Element-Owner $focus
  $focusLog+=@{owner=$owner;shell=($owner -eq $shellPid);name=$(if($owner -eq $shellPid){$focus.Current.Name}else{'non-shell'});class=$focus.Current.ClassName}
  if($owner -ne $shellPid){break}
  if($focus.Current.Name -eq 'Codex Pencil'){$icon=$focus;break}
  $key=if($focus.Current.Name -in @('Show hidden icons','숨겨진 아이콘 표시')){0x0D}else{0x27}
  [TrayInput]::keybd_event($key,0,0,[UIntPtr]::Zero);[TrayInput]::keybd_event($key,0,2,[UIntPtr]::Zero)
  Start-Sleep -Milliseconds 150
 }
 $focusLog|ConvertTo-Json -Depth 3|Set-Content C:\GrammarOutput\tray-focus-diagnostic.json -Encoding utf8
}
if(-not $icon){throw 'Owned tray icon unavailable'}
Click-Verified $icon $shellPid
$quit=$null;$watch=[Diagnostics.Stopwatch]::StartNew()
while(-not $quit -and $watch.ElapsedMilliseconds -lt 5000){$quit=Find-Named 'Quit' $AppPid;if(-not $quit){Start-Sleep -Milliseconds 100}}
if(-not $quit){throw 'Owned Quit menu unavailable'}
$requested=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
Click-Verified $quit $AppPid
@{status='QUIT_CLICKED';appPid=$AppPid;requestedUnixMs=$requested;method='owned-shell-tray-and-app-menu-mouse'}|ConvertTo-Json -Compress
