param([string]$Directory)
$ErrorActionPreference = 'Stop'
try {
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $acl.SetOwner($sid)
    $acl.SetAccessRuleProtection($true,$false)
    $rule = New-Object Security.AccessControl.FileSystemAccessRule($sid,'FullControl','ContainerInherit,ObjectInherit','None','Allow')
    $acl.AddAccessRule($rule)
    [IO.Directory]::SetAccessControl($Directory,$acl)
    exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
