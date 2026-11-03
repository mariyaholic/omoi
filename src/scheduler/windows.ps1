param([ValidateSet('install','remove','status','run')][string]$Mode,[string]$XmlPath,[ValidatePattern('^[A-Za-z0-9-]+$')][string]$FolderName='Omoi')
$ErrorActionPreference = 'Stop'
try {
    $service = New-Object -ComObject 'Schedule.Service'
    $service.Connect()
    $root = $service.GetFolder('\')
    try { $folder = $service.GetFolder('\'+$FolderName) }
    catch {
        $errorCode = $_.Exception.HResult
        if ($_.Exception.InnerException) { $errorCode = $_.Exception.InnerException.HResult }
        if ($errorCode -ne -2147024894) { throw }
        if ($Mode -ne 'install') { if ($Mode -eq 'run') { throw 'Omoi task is not installed' }; Write-Output 'Omoi task is not installed'; exit 0 }
        $folder = $root.CreateFolder($FolderName)
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
            if ($Mode -eq 'run') { throw 'Omoi task is not installed' }
            if ($Mode -eq 'remove' -and $FolderName -like 'OmoiVerify-*' -and $folder.GetTasks(1).Count -eq 0 -and $folder.GetFolders(0).Count -eq 0) { $root.DeleteFolder($FolderName,0) }
            Write-Output 'Omoi task is not installed'; exit 0
        }
        if ($Mode -eq 'remove') {
            $folder.DeleteTask('Collect',0)
            if ($FolderName -like 'OmoiVerify-*' -and $folder.GetTasks(1).Count -eq 0 -and $folder.GetFolders(0).Count -eq 0) { $root.DeleteFolder($FolderName,0) }
            Write-Output 'Omoi task removed'
        }
        elseif ($Mode -eq 'run') { $null = $task.Run($null); Write-Output 'Omoi task started' }
        else { Write-Output $task.Xml; Write-Output ('Last result: '+$task.LastTaskResult); Write-Output ('Last run: '+$task.LastRunTime) }
    }
    exit 0
} catch { [Console]::Error.WriteLine('Omoi Task Scheduler operation failed: '+$_.Exception.Message); exit 1 }
