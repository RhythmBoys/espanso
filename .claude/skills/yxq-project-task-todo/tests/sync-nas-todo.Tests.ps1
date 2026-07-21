$scriptPath = Join-Path $PSScriptRoot '..\scripts\sync-nas-todo.ps1'
if (Test-Path -LiteralPath $scriptPath) {
    . $scriptPath
}

$baseTodo = @'
---
type: todo
tags: [待办事项, 全局]
created_at: 2026-04-07
updated_at: 2026-07-16
updated_by: Peter
source: claude-code
custom_field: keep-me
---

# 全局待办

## 其他项目
- [ ] 保留 [[Wiki链接]]
'@

$sectionV1 = @'
## espanso（yxq-project-task-todo）

### P1 — 当前任务
- [ ] 创建 skill 👤 yxq
'@

$sectionV2 = @'
## espanso（yxq-project-task-todo）

### P1 — 当前任务
- [x] 创建 skill 👤 yxq
'@

Describe 'Merge-NasTodoSection' {
    It 'appends one project section and preserves unrelated content' {
        $merged = Merge-NasTodoSection -ExistingContent $baseTodo -SectionContent $sectionV1 -UpdatedBy 'yxq' -Date '2026-07-20'
        ([regex]::Matches($merged, '(?m)^## espanso（yxq-project-task-todo）\r?$')).Count | Should Be 1
        $merged.Contains('## 其他项目') | Should Be $true
        $merged.Contains('[[Wiki链接]]') | Should Be $true
        $merged.Contains('custom_field: keep-me') | Should Be $true
        $merged.Contains('updated_at: 2026-07-20') | Should Be $true
        $merged.Contains('updated_by: yxq') | Should Be $true
        $merged.Contains('source: Codex') | Should Be $true
    }

    It 'replaces only the exact target section' {
        $first = Merge-NasTodoSection -ExistingContent $baseTodo -SectionContent $sectionV1 -UpdatedBy 'yxq' -Date '2026-07-20'
        $second = Merge-NasTodoSection -ExistingContent $first -SectionContent $sectionV2 -UpdatedBy 'yxq' -Date '2026-07-21'
        $second.Contains('- [x] 创建 skill') | Should Be $true
        $second.Contains('- [ ] 创建 skill') | Should Be $false
        $second.Contains('## 其他项目') | Should Be $true
    }

    It 'is byte-for-byte idempotent for the same section and metadata' {
        $first = Merge-NasTodoSection -ExistingContent $baseTodo -SectionContent $sectionV1 -UpdatedBy 'yxq' -Date '2026-07-20'
        $second = Merge-NasTodoSection -ExistingContent $first -SectionContent $sectionV1 -UpdatedBy 'yxq' -Date '2026-07-20'
        $second | Should BeExactly $first
    }

    It 'rejects empty remote content' {
        { Merge-NasTodoSection -ExistingContent '' -SectionContent $sectionV1 -UpdatedBy 'yxq' } | Should Throw
    }

    It 'rejects content without frontmatter' {
        { Merge-NasTodoSection -ExistingContent '# no frontmatter' -SectionContent $sectionV1 -UpdatedBy 'yxq' } | Should Throw
    }

    It 'rejects duplicate exact target sections' {
        $duplicate = $baseTodo + "`n" + $sectionV1 + "`n" + $sectionV1
        { Merge-NasTodoSection -ExistingContent $duplicate -SectionContent $sectionV2 -UpdatedBy 'yxq' } | Should Throw
    }

    It 'rejects a section that does not start with one H2 heading' {
        { Merge-NasTodoSection -ExistingContent $baseTodo -SectionContent '### wrong level' -UpdatedBy 'yxq' } | Should Throw
    }

    It 'adds required metadata when the source field is absent' {
        $withoutSource = $baseTodo -replace '(?m)^source: .+\r?\n', ''
        $merged = Merge-NasTodoSection -ExistingContent $withoutSource -SectionContent $sectionV1 -UpdatedBy 'yxq' -Date '2026-07-20'
        ([regex]::Matches($merged, '(?m)^source: Codex\r?$')).Count | Should Be 1
        $merged.Contains('custom_field: keep-me') | Should Be $true
    }
}

Describe 'Sync-NasTodo failure protection' {
    It 'keeps the local candidate and throws when the remote hash differs' {
        $sectionPath = Join-Path $TestDrive 'section.md'
        [IO.File]::WriteAllText($sectionPath, $sectionV1, [Text.UTF8Encoding]::new($true))
        Mock Invoke-CheckedCommand {
            if ($FilePath -eq 'ssh' -and $Arguments[1] -like 'cat *') {
                return $baseTodo -split "`r?`n"
            }
            if ($FilePath -eq 'ssh') {
                return 'deadbeef  todo.md'
            }
            return @()
        }

        { Sync-NasTodo -SectionPath $sectionPath -UpdatedBy 'yxq' -Confirm:$false } | Should Throw
        Test-Path -LiteralPath "$sectionPath.candidate.md" | Should Be $true
    }

    It 'removes the candidate after a verified successful sync' {
        $sectionPath = Join-Path $TestDrive 'successful-section.md'
        [IO.File]::WriteAllText($sectionPath, $sectionV1, [Text.UTF8Encoding]::new($true))
        Mock Invoke-CheckedCommand {
            if ($FilePath -eq 'ssh' -and $Arguments[1] -like 'cat *') {
                return $baseTodo -split "`r?`n"
            }
            if ($FilePath -eq 'ssh') {
                $hash = Get-FileSha256Lower -Path "$sectionPath.candidate.md"
                return "$hash  todo.md"
            }
            return @()
        }

        $result = Sync-NasTodo -SectionPath $sectionPath -UpdatedBy 'yxq' -Confirm:$false
        $result.Applied | Should Be $true
        Test-Path -LiteralPath "$sectionPath.candidate.md" | Should Be $false
    }
}
