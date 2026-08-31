// Links against system HarfBuzz (pkg-config). FreeType is pulled in by freetype-rs.
fn main() {
    println!("cargo:rustc-link-lib=harfbuzz");
    if let Ok(lib) = pkg_config_lib_path() {
        println!("cargo:rustc-link-search=native={lib}");
    }
}

fn pkg_config_lib_path() -> Result<String, ()> {
    let out = std::process::Command::new("pkg-config")
        .args(["--libs-only-L", "harfbuzz"])
        .output()
        .map_err(|_| ())?;
    if !out.status.success() {
        return Err(());
    }
    let s = String::from_utf8_lossy(&out.stdout);
    for tok in s.split_whitespace() {
        if let Some(path) = tok.strip_prefix("-L") {
            return Ok(path.to_string());
        }
    }
    Err(())
}
