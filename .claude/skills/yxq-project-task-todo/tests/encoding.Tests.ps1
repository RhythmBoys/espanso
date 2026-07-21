Describe 'Windows PowerShell encoding compatibility' {
    It 'stores every PowerShell file as UTF-8 with BOM' {
        $skillRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
        Get-ChildItem -LiteralPath $skillRoot -Recurse -File -Filter '*.ps1' | ForEach-Object {
            $bytes = [IO.File]::ReadAllBytes($_.FullName)
            ($bytes.Length -ge 3) | Should Be $true
            $bytes[0] | Should Be 0xEF
            $bytes[1] | Should Be 0xBB
            $bytes[2] | Should Be 0xBF
        }
    }

    It 'stores agents metadata as valid UTF-8 without replacement characters' {
        $metadataPath = Join-Path $PSScriptRoot '..\agents\openai.yaml'
        $bytes = [IO.File]::ReadAllBytes($metadataPath)
        $strictUtf8 = [Text.UTF8Encoding]::new($false, $true)
        $text = $strictUtf8.GetString($bytes)
        $text.Contains([char]0xFFFD) | Should Be $false
        $text.Contains('管理项目任务队列') | Should Be $true
    }
}
