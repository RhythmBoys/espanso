$scriptPath = Join-Path $PSScriptRoot '..\scripts\todo.ps1'
if (Test-Path -LiteralPath $scriptPath) {
    . $scriptPath
}

Describe 'Get-NextTodoNumber' {
    It 'starts at 01 for an empty directory' {
        $directory = Join-Path $TestDrive 'empty'
        New-Item -ItemType Directory -Path $directory | Out-Null
        Get-NextTodoNumber -TodoDirectory $directory | Should Be '01'
    }

    It 'uses the maximum canonical number plus one without filling gaps' {
        $directory = Join-Path $TestDrive 'numbered'
        New-Item -ItemType Directory -Path $directory | Out-Null
        New-Item -ItemType File -Path (Join-Path $directory 'todolist_01.md') | Out-Null
        New-Item -ItemType File -Path (Join-Path $directory 'todolist_03.md') | Out-Null
        New-Item -ItemType File -Path (Join-Path $directory 'todolist_03_bug.md') | Out-Null
        New-Item -ItemType File -Path (Join-Path $directory 'todolist_100.md') | Out-Null
        Get-NextTodoNumber -TodoDirectory $directory | Should Be '101'
    }
}

Describe 'New-ProjectTodo' {
    It 'preserves original input and creates a numbered working list' {
        $directory = Join-Path $TestDrive 'create'
        $original = "第一行`n第二行，保留标点。"
        $path = New-ProjectTodo -TodoDirectory $directory -OriginalInput $original -Tasks @('第一项', '第二项') -Date '2026-07-20'
        Split-Path -Leaf $path | Should Be 'todolist_01.md'
        $content = [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8)
        $content.Contains($original) | Should Be $true
        $content.Contains('> 状态: ⬜ 待开始 (0/2)') | Should Be $true
        $content.Contains('- ⬜ 1. 第一项') | Should Be $true
        $content.Contains('- ⬜ 2. 第二项') | Should Be $true
    }

    It 'never overwrites an allocated file' {
        $directory = Join-Path $TestDrive 'collision'
        New-Item -ItemType Directory -Path $directory | Out-Null
        New-Item -ItemType File -Path (Join-Path $directory 'todolist_01.md') | Out-Null
        { New-ProjectTodo -TodoDirectory $directory -OriginalInput 'x' -Tasks @('x') -Number '01' } | Should Throw
    }
}

Describe 'task state updates' {
    It 'updates only the working blocks and preserves original input bytes' {
        $directory = Join-Path $TestDrive 'state'
        $path = New-ProjectTodo -TodoDirectory $directory -OriginalInput "原始`n输入" -Tasks @('任务一', '任务二') -Date '2026-07-20'
        $before = Get-ProjectTodoState -Path $path
        Set-ProjectTodoTaskState -Path $path -TaskNumber 1 -State '完成' -Branch 'feat/example' -Date '2026-07-21'
        $after = Get-ProjectTodoState -Path $path
        $after.OriginalInputHash | Should Be $before.OriginalInputHash
        $after.Completed | Should Be 1
        $after.Total | Should Be 2
        $after.FileState | Should Be '进行中'
        $after.Branch | Should Be 'feat/example'
    }

    It 'marks the file complete after every task completes' {
        $directory = Join-Path $TestDrive 'complete'
        $path = New-ProjectTodo -TodoDirectory $directory -OriginalInput '原始' -Tasks @('唯一任务') -Date '2026-07-20'
        Set-ProjectTodoTaskState -Path $path -TaskNumber 1 -State '完成' -Date '2026-07-21'
        $state = Get-ProjectTodoState -Path $path
        $state.FileState | Should Be '已完成'
        $state.Completed | Should Be 1
    }
}
