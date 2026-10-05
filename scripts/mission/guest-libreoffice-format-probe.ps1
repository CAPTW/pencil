#Requires -Version 7.0
$ErrorActionPreference = 'Stop'
# Real LibreOffice import/export qualification only; this does not invoke Grammar Accept.
if ($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()) { throw 'Guest only' }
if (([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Medium only' }
$root = 'C:\GrammarLOFixture'
$result = [ordered]@{ status='RUNNING'; scope='LibreOffice real engine synthetic format roundtrip'; grammarAccept='NOT_RUN'; microsoftOffice='NOT_RUN'; hancom='NOT_RUN'; checks=@(); cleanup=$false }
function Save-Result { $result | ConvertTo-Json -Depth 8 | Set-Content C:\GrammarOutput\libreoffice-result.json }
function Run-LO([string[]]$LoArgs, [string]$Tag) {
    $psi = [Diagnostics.ProcessStartInfo]::new('C:\GrammarLibreOffice\App\libreoffice\program\soffice.com')
    $psi.UseShellExecute=$false; $psi.CreateNoWindow=$true
    $psi.RedirectStandardOutput=$true; $psi.RedirectStandardError=$true
    $psi.ArgumentList.Add('-env:UserInstallation=file:///C:/GrammarLOFixture/profile')
    foreach ($a in $LoArgs) { $psi.ArgumentList.Add($a) }
    $p=[Diagnostics.Process]::Start($psi)
    $stdout=$p.StandardOutput.ReadToEndAsync(); $stderr=$p.StandardError.ReadToEndAsync()
    if (-not $p.WaitForExit(60000)) { $p.Kill($true); $p.WaitForExit(10000) | Out-Null; throw "Owned LibreOffice timeout: $Tag" }
    if (-not [Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($stdout,$stderr),5000)) { throw "LibreOffice output drain timeout: $Tag" }
    $stdout.GetAwaiter().GetResult() | Set-Content "C:\GrammarOutput\$Tag.log"
    $stderr.GetAwaiter().GetResult() | Set-Content "C:\GrammarOutput\$Tag-error.log"
    if ($p.ExitCode -ne 0) { throw "LibreOffice exit $($p.ExitCode): $Tag" }
    $p.Dispose()
}
function Read-SafeXml([string]$Path) {
    $settings=[Xml.XmlReaderSettings]::new(); $settings.DtdProcessing=[Xml.DtdProcessing]::Prohibit; $settings.XmlResolver=$null
    $reader=[Xml.XmlReader]::Create($Path,$settings)
    try { $doc=[Xml.XmlDocument]::new(); $doc.XmlResolver=$null; $doc.Load($reader); return ,$doc } finally { $reader.Dispose() }
}
try {
    Save-Result
    $extract=Start-Process C:\GrammarLOTools\7z.exe -ArgumentList @('x','C:\GrammarLOTools\engine.paf.exe','App\libreoffice\*','-oC:\GrammarLibreOffice','-y','-bso0','-bsp0') -WindowStyle Hidden -PassThru
    if (-not $extract.WaitForExit(300000)) { $extract.Kill($true); throw 'Owned extraction timeout' }
    if ($extract.ExitCode -ne 0) { throw 'Owned extraction failed' }
    New-Item -ItemType Directory -Path "$root\input","$root\ooxml","$root\roundtrip" | Out-Null
    Run-LO @('--version') 'version'
    $expected='Before | Synthetic English 한국어 😀 | After'
    $escaped=[Security.SecurityElement]::Escape($expected)
    $ns='xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"'
    $cases=@(
        @{flat='fodt'; output='docx'; mime='text'; filter='Office Open XML Text'; back='OpenDocument Text Flat XML'; body="<office:text><text:p>$escaped</text:p></office:text>"},
        @{flat='fods'; output='xlsx'; mime='spreadsheet'; filter='Calc MS Excel 2007 XML'; back='OpenDocument Spreadsheet Flat XML'; body="<office:spreadsheet><table:table table:name='Synthetic'><table:table-row><table:table-cell office:value-type='string'><text:p>$escaped</text:p></table:table-cell></table:table-row></table:table></office:spreadsheet>"},
        @{flat='fodp'; output='pptx'; mime='presentation'; filter='Impress MS PowerPoint 2007 XML'; back='OpenDocument Presentation Flat XML'; body="<office:presentation><draw:page draw:name='page1'><draw:frame svg:x='1cm' svg:y='1cm' svg:width='20cm' svg:height='4cm'><draw:text-box><text:p>$escaped</text:p></draw:text-box></draw:frame></draw:page></office:presentation>"}
    )
    foreach ($case in $cases) {
        $inputFile="$root\input\synthetic.$($case.flat)"
        "<?xml version='1.0' encoding='UTF-8'?><office:document $ns office:version='1.3' office:mimetype='application/vnd.oasis.opendocument.$($case.mime)'><office:body>$($case.body)</office:body></office:document>" | Set-Content $inputFile -Encoding utf8
        # Minimal synthetic XML needs an explicit import filter; auto-detection can import its markup as plain text.
        Run-LO @('--headless','--norestore','--nofirststartwizard',"--infilter=$($case.back)",'--convert-to',"$($case.output):$($case.filter)",'--outdir',"$root\ooxml",$inputFile) "export-$($case.output)"
        $binary="$root\ooxml\synthetic.$($case.output)"
        if (-not (Test-Path $binary)) { throw "No $($case.output) output" }
        Run-LO @('--headless','--norestore','--nofirststartwizard','--convert-to',"$($case.flat):$($case.back)",'--outdir',"$root\roundtrip",$binary) "import-$($case.output)"
        $roundtrip="$root\roundtrip\synthetic.$($case.flat)"
        $doc=Read-SafeXml $roundtrip
        $paragraphs=@($doc.GetElementsByTagName('p','urn:oasis:names:tc:opendocument:xmlns:text:1.0') | ForEach-Object { $_.InnerText })
        if (@($paragraphs | Where-Object { $_ -ceq $expected }).Count -ne 1) { throw "Exact Unicode text mismatch: $($case.output)" }
        $result.checks+=@{ format=$case.output; exactTextRoundtrip=$true; inputSha256=(Get-FileHash $inputFile).Hash; outputSha256=(Get-FileHash $binary).Hash; roundtripSha256=(Get-FileHash $roundtrip).Hash }
        Save-Result
    }
    $result.status='PASS'
} catch { $result.status='FAIL'; $result.error=$_.Exception.Message }
finally {
    # Retain failing synthetic inputs and outputs as well as successful ones.
    foreach ($directory in @('input','ooxml','roundtrip')) {
        if (Test-Path "$root\$directory") { Copy-Item "$root\$directory" C:\GrammarOutput -Recurse }
    }
    # This guest was created solely for this fixture. Never run this cleanup on the host.
    $left=@(Get-Process soffice,soffice.bin -ErrorAction SilentlyContinue)
    $result.remainingProcesses=$left.Count
    if ($left.Count -eq 0) {
        if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
        if (Test-Path -LiteralPath C:\GrammarLibreOffice) { Remove-Item -LiteralPath C:\GrammarLibreOffice -Recurse -Force }
        $result.cleanup=(-not (Test-Path $root)) -and (-not (Test-Path C:\GrammarLibreOffice))
    }
    if (-not $result.cleanup) { $result.status='FAIL' }
    Save-Result
}
if ($result.status -ne 'PASS') { exit 1 }
