# User-assisted input trial. Run only through MediumLaunch in an owned Sandbox.
$ErrorActionPreference='Stop'
if($PSScriptRoot -ne 'C:\GrammarPayload' -or $env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium account required'}
$report=[ordered]@{status='PREPARING';classification='OWNER_ASSISTED_SYNTHETIC_INPUT';physicalKeyboard='NOT_RUN';physicalIME='NOT_RUN';liveProvider='NOT_RUN'}
$app=$null;$editor=$null;$notes=$null
function SaveStatus { $report|ConvertTo-Json -Depth 5|Set-Content C:\GrammarOutput\hands-on.json -Encoding utf8 }
function EditorCommand([string]$line){
 $editor.StandardInput.WriteLine($line);$editor.StandardInput.Flush()
 $read=$editor.StandardOutput.ReadLineAsync()
 if(-not $read.Wait(10000)){throw 'Editor command timed out'}
 if($read.Result -ne 'OK'){throw 'Editor command rejected'}
}
try {
 $manifest=Get-Content C:\GrammarPayload\package-shortcut\MANIFEST.json -Raw|ConvertFrom-Json
 if($manifest.sourceCommit -ne '8e2e0f0270870b6704d263fc3a7e771329e5e94c'){throw 'Unexpected trial package'}
 $report.sourceCommit=$manifest.sourceCommit
 $env:PATH=($env:PATH.Split(';')|Where-Object{$_ -ine 'C:\GrammarPayload\pwsh'}) -join ';'
 $runtime=Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory|Where-Object{Test-Path (Join-Path $_.FullName 'msedgewebview2.exe')}|Sort-Object Name|Select-Object -Last 1
 if(-not $runtime){throw 'Guest WebView2 absent'}
 $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=$runtime.FullName
 $report.explicitGuestRuntime=$runtime.FullName
 & C:\GrammarPayload\package-shortcut\Install-Grammar.ps1 -InstallRoot C:\GrammarTrial | Out-Null
 $config=Join-Path $env:APPDATA 'com.local.codexpencil'
 New-Item -ItemType Directory $config -Force|Out-Null
 $settings=Join-Path $config 'settings.json'
 if(Test-Path $settings){throw 'Expected fresh guest settings'}
 @{schemaVersion=7;autoRewrite=$false;cloudProcessingAcknowledgementVersion=0;antigravityCloudAcknowledgementVersion=0;claudeCloudAcknowledgementVersion=0;onboardingVersion=0;startHiddenToTray=$true;activeProvider='codex';mode='grammar'}|ConvertTo-Json|Set-Content $settings -Encoding utf8
 $psi=[Diagnostics.ProcessStartInfo]::new('C:\GrammarPayload\test-runtime\NativeEditor.exe')
 $psi.UseShellExecute=$false;$psi.RedirectStandardInput=$true;$psi.RedirectStandardOutput=$true;$psi.CreateNoWindow=$true
 $editor=[Diagnostics.Process]::Start($psi)
 $ready=$editor.StandardOutput.ReadLineAsync()
 if(-not $ready.Wait(15000) -or -not $ready.Result.StartsWith('NATIVE_READY:')){throw 'Synthetic editor unavailable'}
 $sample='This is seperate. 한국어 입력 시험입니다.'
 EditorCommand ('SET 0 '+[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($sample)))
 EditorCommand ('SET 5 '+[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes('Paste the copied draft here.')))
 $app=Start-Process C:\GrammarTrial\codex-pencil.exe -PassThru
 Start-Sleep -Seconds 5
 if($app.HasExited){throw 'App exited before trial'}
 $actual=Get-Content $settings -Raw|ConvertFrom-Json
 if($actual.autoRewrite -ne $false -or $actual.cloudProcessingAcknowledgementVersion -ne 0 -or $actual.claudeCloudAcknowledgementVersion -ne 0 -or $actual.antigravityCloudAcknowledgementVersion -ne 0){throw 'Local-only settings mismatch'}
 $report.autoRewrite=$actual.autoRewrite;$report.deepConsent='NONE'
 $report.appPid=$app.Id;$report.editorPid=$editor.Id
 $report.guestLanguageTags=@(Get-WinUserLanguageList|ForEach-Object LanguageTag|Where-Object{$_})
 @'
Grammar — 격리 수동 입력 시험 (사용자 문서 대신 아래 합성 문장만 사용)

1. Grammar 안내가 나오면 Continue 후 창을 닫습니다. 로그인/Deep 동의는 하지 않습니다.
2. Synthetic Native Edit Target 창 맨 위 입력칸에서 문장을 선택합니다.
3. 실제 키보드로 Ctrl+Shift+G를 누릅니다. Instant 결과가 보이는지 확인합니다.
4. 결과 초안을 직접 고쳐 Copy를 누릅니다. 원래 입력칸은 바뀌면 안 됩니다.
5. Synthetic Other Editor 창에 Ctrl+V로 직접 붙여넣어 초안과 일치하는지 확인합니다.
6. 맨 위 원문 칸에서 한글 IME로 '오늘은 입력 시험입니다.'를 직접 입력합니다.
   조합 중 글자가 사라지거나 중복되지 않는지 확인하고, 확정 후 다시 선택/단축키/Copy를 시험합니다.
   한글 입력기를 사용할 수 없다면 설치하지 말고 'IME 사용 불가'라고 보고합니다.
7. 단축키로 닫고 다시 캡처하는 동작을 3회 확인합니다.

이 창은 안내 전용입니다. 새 메모장은 Grammar의 지원 편집기가 아닙니다.
결과를 Codex 대화에 알려 주세요: 단축키 / 초안 편집·Copy / 원문 유지 / 한글 조합 / 재캡처.
실제 입력 문장이나 클립보드 내용은 자동 저장하지 않습니다. 시험 후 Sandbox를 닫아도 됩니다.
세션은 최대 60분이며, 시간이 끝나면 이 시험의 앱·편집기를 종료하고 설치를 제거합니다.
'@ | Set-Content C:\Users\WDAGUtilityAccount\Desktop\Grammar-Trial.txt -Encoding utf8
 $notes=Start-Process C:\GrammarPayload\pwsh\pwsh.exe -ArgumentList '-NoProfile -STA -File C:\GrammarPayload\Guest-Trial-Guide.ps1' -WindowStyle Hidden -PassThru
 $report.status='READY_AWAITING_OWNER_INPUT';$report.startedUtc=[DateTime]::UtcNow.ToString('o');SaveStatus
 $watch=[Diagnostics.Stopwatch]::StartNew()
 while($watch.Elapsed.TotalMinutes -lt 60 -and -not (Test-Path C:\GrammarOutput\finish.request)){
  if($app.HasExited -or $editor.HasExited){break}
  Start-Sleep -Seconds 2
 }
 $report.status='SESSION_ENDED_NO_AUTOMATIC_ACCEPTANCE'
}catch{$report.status='FAIL';$report.error=$_.Exception.Message}
finally{
 foreach($p in @($app,$editor,$notes)){if($p -and -not $p.HasExited){$p.Kill($true);$null=$p.WaitForExit(5000)}}
 if(Test-Path C:\GrammarTrial\install-receipt.json){try{& C:\GrammarTrial\Uninstall-Grammar.ps1 -InstallRoot C:\GrammarTrial -RemoveUserData|Set-Content C:\GrammarOutput\uninstall.json}catch{$report.cleanupError=$_.Exception.Message}}
 SaveStatus
}
