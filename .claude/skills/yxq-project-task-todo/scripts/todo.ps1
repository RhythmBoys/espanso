Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Write-Utf8File {
    param(
        [Parameter(Mandatory = $true)][string]$LiteralPath,
        [Parameter(Mandatory = $true)][string]$Content
    )

    [IO.File]::WriteAllText($LiteralPath, $Content, [Text.UTF8Encoding]::new($true))
}

function Get-TextSha256 {
    param([Parameter(Mandatory = $true)][string]$Text)

    $bytes = [Text.UTF8Encoding]::new($false).GetBytes($Text)
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha.Dispose()
    }
}

function Get-NextTodoNumber {
    param([Parameter(Mandatory = $true)][string]$TodoDirectory)

    if (-not (Test-Path -LiteralPath $TodoDirectory -PathType Container)) {
        return '01'
    }

    $numbers = Get-ChildItem -LiteralPath $TodoDirectory -File -ErrorAction Stop |
        ForEach-Object {
            if ($_.Name -match '^todolist_(\d+)\.md$') {
                [int64]$Matches[1]
            }
        }

    $maximum = 0L
    if ($null -ne $numbers) {
        $maximum = [int64](($numbers | Measure-Object -Maximum).Maximum)
    }

    $next = $maximum + 1
    if ($next -lt 100) {
        return $next.ToString('00')
    }
    return $next.ToString()
}

function New-ProjectTodo {
    param(
        [Parameter(Mandatory = $true)][string]$TodoDirectory,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$OriginalInput,
        [Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string[]]$Tasks,
        [string]$Number,
        [string]$Date = (Get-Date -Format 'yyyy-MM-dd')
    )

    if (-not (Test-Path -LiteralPath $TodoDirectory -PathType Container)) {
        New-Item -ItemType Directory -Path $TodoDirectory -Force | Out-Null
    }

    if ([string]::IsNullOrWhiteSpace($Number)) {
        $Number = Get-NextTodoNumber -TodoDirectory $TodoDirectory
    }
    if ($Number -notmatch '^\d{2,}$') {
        throw "Todo number must contain at least two digits: $Number"
    }

    $path = Join-Path $TodoDirectory "todolist_$Number.md"
    if (Test-Path -LiteralPath $path) {
        throw "Todo file already exists: $path"
    }

    $workingLines = for ($index = 0; $index -lt $Tasks.Count; $index++) {
        "- ⬜ $($index + 1). $($Tasks[$index])"
    }

    $content = @(
        '## 原始输入（原文，勿改）'
        $OriginalInput
        ''
        "> 状态: ⬜ 待开始 (0/$($Tasks.Count))"
        '> 分支: (待执行)'
        "> 更新: $Date"
        ''
        ($workingLines -join "`r`n")
        ''
        '## 说明与上下文（完成后补）'
        '- 做了什么：'
        '- 关键决策：'
        '- 涉及文件：'
        '- 验证结果：'
        '- 风险与后续：'
    ) -join "`r`n"

    Write-Utf8File -LiteralPath $path -Content ($content + "`r`n")
    return (Resolve-Path -LiteralPath $path).Path
}

function Get-ProjectTodoState {
    param([Parameter(Mandatory = $true)][string]$Path)

    $content = [IO.File]::ReadAllText((Resolve-Path -LiteralPath $Path).Path, [Text.Encoding]::UTF8)
    $originalMatch = [regex]::Match(
        $content,
        '(?s)\A## 原始输入（原文，勿改）\r?\n(.*?)\r?\n\r?\n> 状态:'
    )
    if (-not $originalMatch.Success) {
        throw "Invalid todo format: original input block is missing in $Path"
    }

    $statusMatch = [regex]::Match(
        $content,
        '(?m)^> 状态: (?<icon>⬜|🔄|⏸️|✅) (?<state>待开始|进行中|阻塞|已完成) \((?<done>\d+)/(?<total>\d+)\)\r?$'
    )
    if (-not $statusMatch.Success) {
        throw "Invalid todo format: status block is missing in $Path"
    }

    $branchMatch = [regex]::Match($content, '(?m)^> 分支: (?<branch>.+?)\r?$')
    if (-not $branchMatch.Success) {
        throw "Invalid todo format: branch field is missing in $Path"
    }

    [pscustomobject]@{
        Path = (Resolve-Path -LiteralPath $Path).Path
        OriginalInput = $originalMatch.Groups[1].Value
        OriginalInputHash = Get-TextSha256 -Text $originalMatch.Groups[1].Value
        FileState = $statusMatch.Groups['state'].Value
        Completed = [int]$statusMatch.Groups['done'].Value
        Total = [int]$statusMatch.Groups['total'].Value
        Branch = $branchMatch.Groups['branch'].Value
    }
}

function Set-ProjectTodoTaskState {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][ValidateRange(1, 999999)][int]$TaskNumber,
        [Parameter(Mandatory = $true)][ValidateSet('待办', '进行中', '阻塞', '完成')][string]$State,
        [string]$Branch,
        [string]$Date = (Get-Date -Format 'yyyy-MM-dd')
    )

    $resolvedPath = (Resolve-Path -LiteralPath $Path).Path
    $content = [IO.File]::ReadAllText($resolvedPath, [Text.Encoding]::UTF8)
    $before = Get-ProjectTodoState -Path $resolvedPath
    $markerByState = @{ '待办' = '⬜'; '进行中' = '🔄'; '阻塞' = '⏸️'; '完成' = '✅' }
    $taskPattern = "(?m)^- (⬜|🔄|⏸️|✅) $TaskNumber\. "
    $matches = [regex]::Matches($content, $taskPattern)
    if ($matches.Count -ne 1) {
        throw "Expected exactly one working task numbered $TaskNumber in $Path; found $($matches.Count)"
    }

    $content = [regex]::Replace(
        $content,
        $taskPattern,
        "- $($markerByState[$State]) $TaskNumber. ",
        1
    )

    $workingMatches = [regex]::Matches($content, '(?m)^- (?<marker>⬜|🔄|⏸️|✅) \d+\. ')
    $total = $workingMatches.Count
    $completed = @($workingMatches | Where-Object { $_.Groups['marker'].Value -eq '✅' }).Count
    $blocked = @($workingMatches | Where-Object { $_.Groups['marker'].Value -eq '⏸️' }).Count
    if ($completed -eq $total) {
        $fileIcon = '✅'
        $fileState = '已完成'
    }
    elseif ($blocked -gt 0) {
        $fileIcon = '⏸️'
        $fileState = '阻塞'
    }
    elseif ($completed -gt 0 -or $State -eq '进行中') {
        $fileIcon = '🔄'
        $fileState = '进行中'
    }
    else {
        $fileIcon = '⬜'
        $fileState = '待开始'
    }

    $content = [regex]::Replace(
        $content,
        '(?m)^> 状态: .+? \(\d+/\d+\)\r?$',
        "> 状态: $fileIcon $fileState ($completed/$total)"
    )
    if (-not [string]::IsNullOrWhiteSpace($Branch)) {
        $content = [regex]::Replace($content, '(?m)^> 分支: .+?\r?$', "> 分支: $Branch")
    }
    $content = [regex]::Replace($content, '(?m)^> 更新: .+?\r?$', "> 更新: $Date")

    Write-Utf8File -LiteralPath $resolvedPath -Content $content
    $after = Get-ProjectTodoState -Path $resolvedPath
    if ($after.OriginalInputHash -ne $before.OriginalInputHash) {
        throw "Original input changed while updating $Path"
    }
    return $after
}
