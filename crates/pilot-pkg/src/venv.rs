//! Milestone 8 & Milestone 43.3: Universal Pilot Virtual Environment Manager (`pilot venv`).

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const PILOT_VENV_CFG: &str = "pilotvenv.cfg";
pub const PILOT_VENV_ENV_VAR: &str = "PILOT_VENV";

/// Configuration settings read from or written to `pilotvenv.cfg`.
#[derive(Debug, Clone, PartialEq)]
pub struct PilotVenvConfig {
    pub home: String,
    pub version: String,
    pub pilot_version: String,
    pub prompt: String,
    pub include_system_packages: bool,
    pub properties: HashMap<String, String>,
}

impl Default for PilotVenvConfig {
    fn default() -> Self {
        Self {
            home: std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            version: "1.1.0".to_string(),
            pilot_version: "1.1.0".to_string(),
            prompt: ".venv".to_string(),
            include_system_packages: false,
            properties: HashMap::new(),
        }
    }
}

impl PilotVenvConfig {
    pub fn to_cfg_string(&self) -> String {
        let mut s = String::new();
        s.push_str("# Pilot Virtual Environment Configuration\n");
        s.push_str(&format!("home = {}\n", self.home));
        s.push_str(&format!("version = {}\n", self.version));
        s.push_str(&format!("pilot_version = {}\n", self.pilot_version));
        s.push_str(&format!("prompt = {}\n", self.prompt));
        s.push_str(&format!("include_system_packages = {}\n", self.include_system_packages));
        for (k, v) in &self.properties {
            s.push_str(&format!("{} = {}\n", k, v));
        }
        s
    }

    pub fn parse(content: &str) -> Self {
        let mut cfg = Self::default();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            if let Some(eq_idx) = trimmed.find('=') {
                let k = trimmed[..eq_idx].trim().to_ascii_lowercase();
                let v = trimmed[eq_idx + 1..].trim().to_string();
                match k.as_str() {
                    "home" => cfg.home = v,
                    "version" => cfg.version = v,
                    "pilot_version" => cfg.pilot_version = v,
                    "prompt" => cfg.prompt = v,
                    "include_system_packages" => cfg.include_system_packages = v == "true",
                    _ => { cfg.properties.insert(k, v); }
                }
            }
        }
        cfg
    }
}

/// Creates a new virtual environment at `venv_dir` with cross-platform activation scripts.
pub fn init_venv(venv_dir: &str, prompt: Option<&str>, include_system_packages: bool) -> io::Result<PathBuf> {
    let root = Path::new(venv_dir);
    let bin_dir = root.join("bin");
    let lib_dir = root.join("lib").join("pilot_modules");
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&lib_dir)?;

    let prompt_name = prompt.unwrap_or_else(|| {
        root.file_name().and_then(|n| n.to_str()).unwrap_or(".venv")
    });

    // 1. pilotvenv.cfg
    let mut config = PilotVenvConfig::default();
    config.prompt = prompt_name.to_string();
    config.include_system_packages = include_system_packages;
    fs::write(root.join(PILOT_VENV_CFG), config.to_cfg_string())?;

    // 2. POSIX bash/zsh activation script: bin/activate
    let bash_activate = format!(
        r#"# Pilot Virtual Environment Activation Script
deactivate () {{
    if [ -n "${{_OLD_PILOT_PATH:-}}" ] ; then
        PATH="${{_OLD_PILOT_PATH:-}}"
        export PATH
        unset _OLD_PILOT_PATH
    fi
    if [ -n "${{_OLD_PILOT_PS1:-}}" ] ; then
        PS1="${{_OLD_PILOT_PS1:-}}"
        export PS1
        unset _OLD_PILOT_PS1
    fi
    unset PILOT_VENV
    if [ ! "${{1:-}}" = "nondestructive" ] ; then
        unset -f deactivate
    fi
}}
deactivate nondestructive
PILOT_VENV="$(cd "$(dirname "${{BASH_SOURCE[0]}}")/.." && pwd)"
export PILOT_VENV
_OLD_PILOT_PATH="$PATH"
PATH="$PILOT_VENV/bin:$PATH"
export PATH
_OLD_PILOT_PS1="${{PS1:-}}"
PS1="({prompt}) ${{PS1:-}}"
export PS1
"#,
        prompt = prompt_name
    );
    fs::write(bin_dir.join("activate"), bash_activate)?;

    // 3. PowerShell activation script: bin/activate.ps1
    let ps1_activate = format!(
        r#"# Pilot Virtual Environment PowerShell Activation Script
function global:deactivate ([switch]$NonDestructive) {{
    if (Test-Path variable:_OLD_PILOT_PATH) {{
        $env:PATH = $variable:_OLD_PILOT_PATH
        Remove-Variable _OLD_PILOT_PATH -Scope global
    }}
    if (Test-Path function:_old_pilot_prompt) {{
        Set-Item function:prompt -Value $function:_old_pilot_prompt -Force
        Remove-Item function:_old_pilot_prompt
    }}
    Remove-Item env:PILOT_VENV -ErrorAction SilentlyContinue
    if (-not $NonDestructive) {{
        Remove-Item function:deactivate -ErrorAction SilentlyContinue
    }}
}}
deactivate -NonDestructive
$script:VenvRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Definition)
$env:PILOT_VENV = $script:VenvRoot
$global:_OLD_PILOT_PATH = $env:PATH
$env:PATH = "$script:VenvRoot\bin;$env:PATH"
if (Get-Command prompt -ErrorAction SilentlyContinue) {{
    $function:_old_pilot_prompt = $function:prompt
}}
function global:prompt {{
    Write-Host "({prompt}) " -NoNewline -ForegroundColor Cyan
    return "> "
}}
"#,
        prompt = prompt_name
    );
    fs::write(bin_dir.join("activate.ps1"), ps1_activate)?;

    // 4. Windows batch activation script: bin/activate.bat
    let bat_activate = format!(
        r#"@echo off
set "PILOT_VENV=%~dp0.."
set "_OLD_PILOT_PATH=%PATH%"
set "PATH=%PILOT_VENV%\bin;%PATH%"
set "_OLD_PILOT_PROMPT=%PROMPT%"
set "PROMPT=({prompt}) %PROMPT%"
"#,
        prompt = prompt_name
    );
    fs::write(bin_dir.join("activate.bat"), bat_activate)?;

    Ok(root.to_path_buf())
}

/// Returns the currently active virtual environment directory path if set.
pub fn get_active_venv() -> Option<PathBuf> {
    std::env::var(PILOT_VENV_ENV_VAR).ok().map(PathBuf::from)
}
