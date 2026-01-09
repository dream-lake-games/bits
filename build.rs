use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let mut src_anims: Vec<(String, String)> = Vec::new();
    let mut example_anims: HashMap<String, Vec<String>> = HashMap::new();

    scan_dir(Path::new("src"), "crate", &mut src_anims);

    if let Ok(entries) = fs::read_dir("examples") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let example_name = path.file_name().unwrap().to_string_lossy().to_string();
                let mut anims = Vec::new();
                scan_example_dir(&path, &mut anims);
                if !anims.is_empty() {
                    example_anims.insert(example_name, anims);
                }
            }
        }
    }

    let src_registrations: String = src_anims
        .iter()
        .map(|(module_path, name)| {
            format!(
                "    crate::bits_ui::anim::register_anim::<{}::{}>(__app);",
                module_path, name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let src_code = format!(
        r#"#[allow(unused)]
pub fn register_all_anims(__app: &mut bevy::prelude::App) {{
{src_registrations}
}}"#
    );

    fs::write(Path::new(&out_dir).join("anim_registry.rs"), src_code).unwrap();

    for (example_name, anims) in &example_anims {
        let registrations: String = anims
            .iter()
            .map(|name| format!("    bits::bits_ui::anim::register_anim::<{}>(__app);", name))
            .collect::<Vec<_>>()
            .join("\n");

        let code = format!(
            r#"#[allow(unused)]
pub fn register_anims(__app: &mut bevy::prelude::App) {{
{registrations}
}}"#
        );

        fs::write(
            Path::new(&out_dir).join(format!("anim_registry_{}.rs", example_name)),
            code,
        )
        .unwrap();
    }

    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=examples");
}

fn scan_dir(dir: &Path, module_prefix: &str, results: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_string_lossy();
            let new_prefix = format!("{}::{}", module_prefix, dir_name);
            scan_dir(&path, &new_prefix, results);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                for name in find_anim_enums(&content) {
                    let file_stem = path.file_stem().unwrap().to_string_lossy();
                    let mod_path = if file_stem == "mod" || file_stem == "lib" {
                        module_prefix.to_string()
                    } else {
                        format!("{}::{}", module_prefix, file_stem)
                    };
                    results.push((mod_path, name));
                }
            }
        }
    }
}

fn scan_example_dir(dir: &Path, results: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                for name in find_anim_enums(&content) {
                    results.push(name);
                }
            }
        }
    }
}

fn find_anim_enums(content: &str) -> Vec<String> {
    let mut results = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.contains("#[derive(") && line.contains("Anim") {
            if let Some(enum_line) = find_enum_after(i + 1, &lines) {
                if let Some(name) = extract_enum_name(enum_line) {
                    results.push(name);
                }
            }
        }
    }

    results
}

fn find_enum_after<'a>(start: usize, lines: &[&'a str]) -> Option<&'a str> {
    for i in start..lines.len().min(start + 10) {
        let line = lines[i].trim();
        if line.starts_with("enum ") || line.starts_with("pub enum ") {
            return Some(line);
        }
        if line.starts_with("struct ") || line.starts_with("pub struct ") {
            return None;
        }
    }
    None
}

fn extract_enum_name(line: &str) -> Option<String> {
    let line = line.trim();
    let line = line.strip_prefix("pub ").unwrap_or(line).trim();
    let line = line.strip_prefix("enum ")?;
    let line = line.trim();
    let name_end = line
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(line.len());
    let name = &line[..name_end];
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

