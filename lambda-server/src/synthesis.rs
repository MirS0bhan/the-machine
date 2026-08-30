//! Code synthesis: write source to disk and produce entrypoint.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub struct SynthesisResult {
    pub entrypoint: String,
}

pub fn write_synthesized_function(
    name: &str,
    language: &str,
    source: &str,
) -> Result<SynthesisResult, String> {
    let base = std::env::var("THE_MACHINE_LAMBDA_DIR")
        .unwrap_or_else(|_| "/var/the-machine/lambdas".to_string());
    write_synthesized_function_in(&base, name, language, source)
}

fn write_synthesized_function_in(
    base: &str,
    name: &str,
    language: &str,
    source: &str,
) -> Result<SynthesisResult, String> {
    let dir = PathBuf::from(base).join(name.replace('.', "_"));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    match language {
        "python" | "py" => {
            let path = dir.join("main.py");
            // Sandbox bind-mounts `entrypoint` as a single file. A shell-style
            // "python3 /path/main.py" string is not a valid mount source.
            let script = if source.starts_with("#!") {
                source.to_string()
            } else {
                format!("#!/usr/bin/env python3\n{source}")
            };
            fs::write(&path, &script).map_err(|e| e.to_string())?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).ok();
            Ok(SynthesisResult {
                entrypoint: path.display().to_string(),
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
            })
        }
        _ => {
            let path = dir.join("main.sh");
            fs::write(&path, source).map_err(|e| e.to_string())?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).ok();
            Ok(SynthesisResult {
                entrypoint: path.display().to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_entrypoint_is_a_single_script_path() {
        let dir = std::env::temp_dir().join(format!("tm-synth-{}", std::process::id()));
        let result = write_synthesized_function_in(
            dir.to_str().unwrap(),
            "calc.eval",
            "python",
            "import json\nprint(json.dumps({'ok': True}))\n",
        )
        .unwrap();
        assert!(
            !result.entrypoint.contains(' '),
            "entrypoint must be one path, got {}",
            result.entrypoint
        );
        assert!(result.entrypoint.ends_with("main.py"));
        let body = fs::read_to_string(&result.entrypoint).unwrap();
        assert!(body.starts_with("#!/usr/bin/env python3"));
        let _ = fs::remove_dir_all(&dir);
    }
}
