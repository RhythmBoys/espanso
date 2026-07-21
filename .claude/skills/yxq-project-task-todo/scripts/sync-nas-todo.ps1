Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Set-FrontmatterField {
    param(
        [Parameter(Mandatory = $true)][string]$Content,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Value,
        [Parameter(Mandatory = $true)][string]$NewLine
    )

    $pattern = "(?m)^$([regex]::Escape($Name)):.*?\r?$"
    if ([regex]::IsMatch($Content, $pattern)) {
        return [regex]::Replace($Content, $pattern, "$Name`: $Value", 1)
    }

    $frontmatterEnd = [regex]::Match($Content, '(?m)^---\r?$', [Text.RegularExpressions.RegexOptions]::None, 4)
    if (-not $frontmatterEnd.Success) {
        throw 'Frontmatter closing delimiter is missing.'
    }
    return $Content.Insert($frontmatterEnd.Index, "$Name`: $Value$NewLine")
}

function Merge-NasTodoSection {
    param(
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ExistingContent,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$SectionContent,
        [Parameter(Mandatory = $true)][string]$UpdatedBy,
        [string]$Date = (Get-Date -Format 'yyyy-MM-dd')
    )

    if ([string]::IsNullOrWhiteSpace($ExistingContent)) {
        throw 'Remote todo.md is empty; refusing to replace it.'
    }
    if ($ExistingContent -notmatch '\A---\r?\n(?s:.*?)\r?\n---\r?\n') {
        throw 'Remote todo.md does not start with valid YAML frontmatter.'
    }
    if ([string]::IsNullOrWhiteSpace($SectionContent)) {
        throw 'Project section is empty.'
    }

    $newLine = if ($ExistingContent.Contains("`r`n")) { "`r`n" } else { "`n" }
    $normalizedSection = (($SectionContent -replace "`r`n", "`n") -replace "`r", "`n").Trim()
    $normalizedSection = $normalizedSection -replace "`n", $newLine
    $firstLineEnd = $normalizedSection.IndexOf($newLine)
    $heading = if ($firstLineEnd -ge 0) {
        $normalizedSection.Substring(0, $firstLineEnd)
    }
    else {
        $normalizedSection
    }
    if ($heading -notmatch '^## [^#].+') {
        throw 'Project section must start with exactly one H2 heading.'
    }
    if (([regex]::Matches($normalizedSection, '(?m)^## [^#].*\r?$')).Count -ne 1) {
        throw 'Project section must contain exactly one H2 heading.'
    }

    $headingPattern = "(?m)^$([regex]::Escape($heading))\r?$"
    $headingMatches = [regex]::Matches($ExistingContent, $headingPattern)
    if ($headingMatches.Count -gt 1) {
        throw "Remote todo.md contains duplicate target sections: $heading"
    }

    if ($headingMatches.Count -eq 0) {
        $merged = $ExistingContent.TrimEnd() + $newLine + $newLine + '---' + $newLine + $newLine + $normalizedSection + $newLine
    }
    else {
        $sectionPattern = "(?ms)^$([regex]::Escape($heading))\r?\n.*?(?=^## [^#]|\z)"
        $sectionMatches = [regex]::Matches($ExistingContent, $sectionPattern)
        if ($sectionMatches.Count -ne 1) {
            throw "Could not isolate target section safely: $heading"
        }
        $match = $sectionMatches[0]
        $prefix = $ExistingContent.Substring(0, $match.Index)
        $suffix = $ExistingContent.Substring($match.Index + $match.Length)
        $merged = $prefix + $normalizedSection + $newLine + $newLine + $suffix.TrimStart("`r", "`n")
        $merged = $merged.TrimEnd() + $newLine
    }

    $merged = Set-FrontmatterField -Content $merged -Name 'updated_at' -Value $Date -NewLine $newLine
    $merged = Set-FrontmatterField -Content $merged -Name 'updated_by' -Value $UpdatedBy -NewLine $newLine
    $merged = Set-FrontmatterField -Content $merged -Name 'source' -Value 'Codex' -NewLine $newLine

    $finalCount = ([regex]::Matches($merged, $headingPattern)).Count
    if ($finalCount -ne 1) {
        throw "Merged todo.md contains $finalCount target sections; expected one."
    }
    return $merged
}

function Get-FileSha256Lower {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Invoke-CheckedCommand {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    $output = & $FilePath @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE`: $($output -join [Environment]::NewLine)"
    }
    return @($output)
}

function Sync-NasTodo {
    [CmdletBinding(SupportsShouldProcess = $true)]
    param(
        [Parameter(Mandatory = $true)][string]$SectionPath,
        [Parameter(Mandatory = $true)][string]$UpdatedBy,
        [string]$Date = (Get-Date -Format 'yyyy-MM-dd'),
        [string]$SshTarget = 'peter-admin@192.168.1.167',
        [string]$RemotePath = '/volume2/Sharefile/Obsidian-KB/AltitudeCraft/todo.md'
    )

    $resolvedSection = (Resolve-Path -LiteralPath $SectionPath).Path
    $section = [IO.File]::ReadAllText($resolvedSection, [Text.Encoding]::UTF8)
    $remoteOutput = Invoke-CheckedCommand -FilePath 'ssh' -Arguments @($SshTarget, "cat '$RemotePath'")
    $existing = $remoteOutput -join "`n"
    $merged = Merge-NasTodoSection -ExistingContent $existing -SectionContent $section -UpdatedBy $UpdatedBy -Date $Date
    $candidatePath = "$resolvedSection.candidate.md"
    [IO.File]::WriteAllText($candidatePath, $merged, [Text.UTF8Encoding]::new($false))

    if (-not $PSCmdlet.ShouldProcess($RemotePath, 'back up and replace NAS todo.md')) {
        return [pscustomobject]@{
            Changed = ($merged -ne $existing)
            CandidatePath = $candidatePath
            RemotePath = $RemotePath
            Applied = $false
        }
    }

    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $remoteDirectory = $RemotePath.Substring(0, $RemotePath.LastIndexOf('/'))
    $remoteTemp = "$remoteDirectory/.todo.md.yxq-project-task-todo.$stamp.tmp"
    $remoteBackup = "$RemotePath.backup-$stamp-yxq-project-task-todo"
    Invoke-CheckedCommand -FilePath 'scp' -Arguments @('-O', $candidatePath, "${SshTarget}:$remoteTemp") | Out-Null

    $remoteCommand = "set -e; test -s '$RemotePath'; test -s '$remoteTemp'; cp -p '$RemotePath' '$remoteBackup'; mv '$remoteTemp' '$RemotePath'; sha256sum '$RemotePath'"
    $hashOutput = Invoke-CheckedCommand -FilePath 'ssh' -Arguments @($SshTarget, $remoteCommand)
    $remoteHash = (($hashOutput -join ' ') -split '\s+')[0].ToLowerInvariant()
    $localHash = Get-FileSha256Lower -Path $candidatePath
    if ($remoteHash -ne $localHash) {
        throw "Remote hash mismatch. Local=$localHash Remote=$remoteHash Backup=$remoteBackup"
    }

    Remove-Item -LiteralPath $candidatePath -Force

    return [pscustomobject]@{
        Changed = ($merged -ne $existing)
        CandidatePath = $null
        CandidateRemoved = $true
        RemotePath = $RemotePath
        BackupPath = $remoteBackup
        Sha256 = $localHash
        Applied = $true
    }
}
