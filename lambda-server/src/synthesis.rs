//! Code synthesis: write source to disk and produce entrypoint.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub struct SynthesisResult {
    pub entrypoint: String,
    pub source_path: String,
}

pub fn write_synthesized_function(name: &str, language: &str, source: &str) -> Result<SynthesisResult, String> {
    let base = std::env::var("THE_MACHINE_LAMBDA_DIR")
        .unwrap_or_else(|_| "/var/the-machine/lambdas".to_string());
    let dir = PathBuf::from(&base).join(name.replace('.', "_"));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    match language {
        "python" | "py" => {
            let path = dir.join("main.py");
            fs::write(&path, source).map_err(|e| e.to_string())?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).ok();
            Ok(SynthesisResult {
                entrypoint: format!("/usr/bin/python3 {}", path.display()),
                source_path: path.display().to_string(),
            })
        }
        "shell" | "sh" => {
            let path = dir.join("main.sh");
            let script = if source.starts_with("#!") {
                source.to_string()
            } else {
                format!("#!/bin/sh\n{}", source)
            };
            fs::write(&path, &script).map_err(|e| e.to_string())?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).ok();
            Ok(SynthesisResult {
                entrypoint: path.display().to_string(),
                source_path: path.display().to_string(),
            })
        }
        _ => {
            let path = dir.join("main.sh");
            fs::write(&path, source).map_err(|e| e.to_string())?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).ok();
            Ok(SynthesisResult {
                entrypoint: path.display().to_string(),
                source_path: path.display().to_string(),
            })
        }
    }
}
