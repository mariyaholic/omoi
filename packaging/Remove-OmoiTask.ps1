# Called by this installer only; delete the task only if its executable is ours.
$ErrorActionPreference='Stop'
$service=New-Object -ComObject 'Schedule.Service'
$service.Connect()
try{$folder=$service.GetFolder('\Omoi')}catch{exit 0}
try{$task=$folder.GetTask('Collect')}catch{exit 0}
[xml]$xml=$task.Xml
$expected=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'omoiw.exe'))
$command=$xml.Task.Actions.Exec.Command
if($command -and [IO.Path]::GetFullPath($command) -eq $expected){$folder.DeleteTask('Collect',0)}
# Never remove the shared folder or another installation's task; preserve data.
