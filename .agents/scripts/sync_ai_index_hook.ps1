#Requires -Version 7.0
<#
.SYNOPSIS
    Antigravity SessionStart & PostToolUse lifecycle hook wrapper.
    Skips read-only tool calls instantly; otherwise runs update_ai_index.ps1 -CheckStampOnly
    so .agents/AI_CODEBASE_INDEX.md stays automatically synchronized after any workspace edit.
#>
$ErrorActionPreference = "SilentlyContinue"

try {
    $rawInput = [Console]::In.ReadToEnd()
    $skip = $false

    if (-not [string]::IsNullOrWhiteSpace($rawInput)) {
        $payload = $rawInput | ConvertFrom-Json -ErrorAction SilentlyContinue
        if ($null -ne $payload -and $null -ne $payload.toolCall -and -not [string]::IsNullOrWhiteSpace($payload.toolCall.name)) {
            $toolName = $payload.toolCall.name.ToLowerInvariant()
            if ($toolName -in @(
                'read_file', 'view_file', 'view_file_outline', 'view_code_item',
                'grep_search', 'find_by_name', 'list_dir', 'search_web',
                'read_url_content', 'list_resources', 'read_resource',
                'ask_question', 'manage_task', 'manage_subagents', 'send_message', 'schedule'
            )) {
                $skip = $true
            }
        }
    }

    if (-not $skip) {
        $updater = Join-Path $PSScriptRoot "update_ai_index.ps1"
        if (Test-Path $updater) {
            & $updater -CheckStampOnly | Out-Null
        }
    }
}
catch {
    # Never block the agent loop on hook errors
}

[Console]::Out.WriteLine("{}")
exit 0
