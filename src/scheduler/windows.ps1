param([ValidateSet('install','remove','status')][string]$Mode,[string]$XmlPath)
$ErrorActionPreference = 'Stop'
try {
    $service = New-Object -ComObject 'Schedule.Service'
    $service.Connect()
    $root = $service.GetFolder('\')
    try { $folder = $service.GetFolder('\Omoi') }
    catch {
        $errorCode = $_.Exception.HResult
        if ($_.Exception.InnerException) { $errorCode = $_.Exception.InnerException.HResult }
        if ($errorCode -ne -2147024894) { throw }
        if ($Mode -ne 'install') { Write-Output 'Omoi task is not installed'; exit 0 }
        $folder = $root.CreateFolder('Omoi')
    }
    if ($Mode -eq 'install') {
        $text = [IO.File]::ReadAllText($XmlPath,[Text.Encoding]::UTF8)
        $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
        $null = $folder.RegisterTask('Collect',$text,6,$sid,$null,3,$null)
        Write-Output ($folder.GetTask('Collect').Xml)
    } else {
        try { $task = $folder.GetTask('Collect') }
        catch {
            $errorCode = $_.Exception.HResult
            if ($_.Exception.InnerException) { $errorCode = $_.Exception.InnerException.HResult }
            if ($errorCode -ne -2147024894) { throw }
            Write-Output 'Omoi task is not installed'; exit 0
        }
        if ($Mode -eq 'remove') { $folder.DeleteTask('Collect',0); Write-Output 'Omoi task removed' }
        else { Write-Output $task.Xml }
    }
    exit 0
} catch { [Console]::Error.WriteLine('Omoi Task Scheduler operation failed: '+$_.Exception.Message); exit 1 }
