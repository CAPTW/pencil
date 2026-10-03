$ErrorActionPreference='Stop'
if($PSScriptRoot -ne 'C:\GrammarPayload' -or $env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$form=[Windows.Forms.Form]::new()
$form.Text='Grammar — 수동 시험 안내 (입력 시험 대상 아님)'
$form.Size=[Drawing.Size]::new(720,520)
$form.StartPosition='CenterScreen'
$text=[Windows.Forms.TextBox]::new()
$text.Multiline=$true;$text.ReadOnly=$true;$text.ScrollBars='Vertical';$text.Dock='Fill'
$text.Font=[Drawing.Font]::new('Malgun Gothic',11)
$text.Text=Get-Content C:\Users\WDAGUtilityAccount\Desktop\Grammar-Trial.txt -Raw
$form.Controls.Add($text)
$form.Add_Shown({[ordered]@{status='GUIDE_SHOWN';physicalKeyboard='NOT_RUN';physicalIME='NOT_RUN'}|ConvertTo-Json|Set-Content C:\GrammarOutput\guide.json -Encoding utf8})
[Windows.Forms.Application]::Run($form)
$form.Dispose()
