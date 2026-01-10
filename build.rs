use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

const ASEPRITE_BIN: &str = "/Users/mork/Library/Application Support/Steam/steamapps/common/Aseprite/Aseprite.app/Contents/MacOS/aseprite";

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let mut src_anims: Vec<AnimInfo> = Vec::new();
    let mut example_anims: HashMap<String, Vec<String>> = HashMap::new();

    scan_dir_for_anims(Path::new("src"), "crate", &mut src_anims);

    if let Ok(entries) = fs::read_dir("examples") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let example_name = path.file_name().unwrap().to_string_lossy().to_string();
                let mut anims: Vec<AnimInfo> = Vec::new();
                scan_example_dir_for_anims(&path, &mut anims);

                // Process example anims
                for anim in &anims {
                    if let Err(e) = process_anim(anim) {
                        eprintln!("Warning: Failed to process anim {}: {}", anim.name, e);
                    }
                }

                if !anims.is_empty() {
                    example_anims
                        .insert(example_name, anims.iter().map(|a| a.name.clone()).collect());
                }
            }
        }
    }

    // Process src anims and generate sprite sheets
    for anim in &src_anims {
        if let Err(e) = process_anim(anim) {
            eprintln!("Warning: Failed to process anim {}: {}", anim.name, e);
        }
    }

    // Generate registration code
    let src_registrations: String = src_anims
        .iter()
        .map(|anim| {
            format!(
                "    crate::bits_ui::anim::register_anim::<{}::{}>(__app);",
                anim.module_path, anim.name
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

    // Rerun triggers
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=examples");
    println!("cargo:rerun-if-changed=assets");
}

#[derive(Debug, Clone)]
struct AnimInfo {
    name: String,
    module_path: String,
    file_path: Option<String>,
    exclude_prefix: Option<String>,
}

fn scan_dir_for_anims(dir: &Path, module_prefix: &str, results: &mut Vec<AnimInfo>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_string_lossy();
            let new_prefix = format!("{}::{}", module_prefix, dir_name);
            scan_dir_for_anims(&path, &new_prefix, results);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                for info in find_anim_enums_with_attrs(&content) {
                    let file_stem = path.file_stem().unwrap().to_string_lossy();
                    let mod_path = if file_stem == "mod" || file_stem == "lib" {
                        module_prefix.to_string()
                    } else {
                        format!("{}::{}", module_prefix, file_stem)
                    };
                    results.push(AnimInfo {
                        name: info.name,
                        module_path: mod_path,
                        file_path: info.file_path,
                        exclude_prefix: info.exclude_prefix,
                    });
                }
            }
        }
    }
}

fn scan_example_dir_for_anims(dir: &Path, results: &mut Vec<AnimInfo>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(content) = fs::read_to_string(&path) {
                for info in find_anim_enums_with_attrs(&content) {
                    results.push(AnimInfo {
                        name: info.name.clone(),
                        module_path: String::new(),
                        file_path: info.file_path,
                        exclude_prefix: info.exclude_prefix,
                    });
                }
            }
        }
    }
}

#[derive(Debug)]
struct ParsedAnimInfo {
    name: String,
    file_path: Option<String>,
    exclude_prefix: Option<String>,
}

fn find_anim_enums_with_attrs(content: &str) -> Vec<ParsedAnimInfo> {
    let mut results = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.contains("#[derive(") && line.contains("Anim") {
            // Look backwards for #[file(...)] and #[exclude_prefix(...)]
            let mut file_path = None;
            let mut exclude_prefix = None;

            for j in (0..i).rev() {
                let attr_line = lines[j].trim();
                if attr_line.starts_with("#[file(") {
                    file_path = extract_string_attr(attr_line, "file");
                } else if attr_line.starts_with("#[exclude_prefix(") {
                    exclude_prefix = extract_string_attr(attr_line, "exclude_prefix");
                } else if !attr_line.starts_with("#[") && !attr_line.is_empty() {
                    break;
                }
            }

            // Also check the derive line itself and lines after for attributes
            for j in i..lines.len().min(i + 5) {
                let attr_line = lines[j].trim();
                if attr_line.starts_with("#[file(") {
                    file_path = extract_string_attr(attr_line, "file");
                } else if attr_line.starts_with("#[exclude_prefix(") {
                    exclude_prefix = extract_string_attr(attr_line, "exclude_prefix");
                }
            }

            if let Some(enum_line) = find_enum_after(i + 1, &lines) {
                if let Some(name) = extract_enum_name(enum_line) {
                    results.push(ParsedAnimInfo {
                        name,
                        file_path,
                        exclude_prefix,
                    });
                }
            }
        }
    }

    results
}

fn extract_string_attr(line: &str, attr_name: &str) -> Option<String> {
    let prefix = format!("#[{}(\"", attr_name);
    if let Some(start) = line.find(&prefix) {
        let rest = &line[start + prefix.len()..];
        if let Some(end) = rest.find("\")") {
            return Some(rest[..end].to_string());
        }
    }
    None
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

// ============================================================================
// Aseprite Processing
// ============================================================================

fn process_anim(anim: &AnimInfo) -> Result<(), String> {
    let Some(ref file_path) = anim.file_path else {
        return Ok(()); // No file path, nothing to process
    };

    if !Path::new(file_path).exists() {
        return Err(format!("Aseprite file not found: {}", file_path));
    }

    let enum_snake = to_snake_case(&anim.name);
    let output_dir = format!("assets/_generated/anim/{}", enum_snake);
    let metadata_path = format!("{}/_metadata.json", output_dir);

    // Check if we need to regenerate
    if is_metadata_valid(file_path, &metadata_path) {
        return Ok(());
    }

    // Create output directory
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    // List tags
    let tags = list_tags(file_path)?;

    // Get layers to ignore
    let layers_to_ignore = if let Some(ref prefix) = anim.exclude_prefix {
        list_layers(file_path)?
            .into_iter()
            .filter(|l| l.starts_with(prefix))
            .collect()
    } else {
        vec![]
    };

    // Export all sprite sheets using Lua script
    let export_results = batch_export(file_path, &tags, &output_dir, &layers_to_ignore)?;

    // Get frame ranges for all tags
    let frame_ranges = get_all_frame_ranges(file_path)?;

    // Write metadata
    write_metadata(&metadata_path, file_path, &export_results, &frame_ranges)?;

    Ok(())
}

fn is_metadata_valid(source_file: &str, metadata_path: &str) -> bool {
    let Ok(metadata_content) = fs::read_to_string(metadata_path) else {
        return false;
    };

    // Parse metadata and check source mtime
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&metadata_content) {
        if let Some(stored_mtime) = json["source_mtime"].as_u64() {
            if let Ok(meta) = fs::metadata(source_file) {
                if let Ok(mtime) = meta.modified() {
                    let current_mtime = mtime
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    return stored_mtime == current_mtime;
                }
            }
        }
    }
    false
}

fn list_tags(file: &str) -> Result<Vec<String>, String> {
    let output = run_aseprite(&["-b", "--list-tags", file])?;
    Ok(output.split_whitespace().map(|s| s.to_string()).collect())
}

fn list_layers(file: &str) -> Result<Vec<String>, String> {
    let output = run_aseprite(&["-b", "--list-layers", file])?;
    Ok(output.lines().map(|s| s.to_string()).collect())
}

fn run_aseprite(args: &[&str]) -> Result<String, String> {
    if !Path::new(ASEPRITE_BIN).exists() {
        return Err(format!("Aseprite not found at: {}", ASEPRITE_BIN));
    }

    let output = Command::new(ASEPRITE_BIN)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run Aseprite: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Aseprite failed: {}", stderr));
    }

    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

#[derive(Debug)]
struct ExportResult {
    tag: String,
    frame_count: usize,
    width: u32,
    height: u32,
}

fn batch_export(
    file: &str,
    tags: &[String],
    output_dir: &str,
    layers_to_ignore: &[String],
) -> Result<Vec<ExportResult>, String> {
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("build_batch_export.lua");
    let json_path = temp_dir.join("build_batch_meta.json");

    let ignore_layers_lua: String = layers_to_ignore
        .iter()
        .map(|l| format!("[\"{}\"] = true", l))
        .collect::<Vec<_>>()
        .join(", ");

    let tags_lua: String = tags
        .iter()
        .map(|t| format!("\"{}\"", t))
        .collect::<Vec<_>>()
        .join(", ");

    let lua_script = format!(
        r#"
local spr = app.sprite
local ignoreLayers = {{ {ignore_layers} }}
local tags = {{ {tags} }}
local outputDir = "{output_dir}"
local results = {{}}

for _, layer in ipairs(spr.layers) do
    if ignoreLayers[layer.name] then
        layer.isVisible = false
    end
end

for _, tagName in ipairs(tags) do
    local tag = nil
    for _, t in ipairs(spr.tags) do
        if t.name == tagName then
            tag = t
            break
        end
    end
    if tag then
        local outputPath = outputDir .. "/" .. tagName .. ".png"
        app.command.ExportSpriteSheet {{
            ui = false,
            type = SpriteSheetType.HORIZONTAL,
            textureFilename = outputPath,
            tag = tagName,
        }}
        local frameCount = tag.frames
        local w = spr.width
        local h = spr.height
        table.insert(results, string.format("%s,%d,%d,%d", tagName, frameCount, w, h))
    end
end

local f = io.open("{json_path}", "w")
f:write(table.concat(results, "\n"))
f:close()
"#,
        ignore_layers = ignore_layers_lua,
        tags = tags_lua,
        output_dir = output_dir.replace('\\', "/"),
        json_path = json_path.to_str().unwrap().replace('\\', "/"),
    );

    fs::write(&script_path, &lua_script).map_err(|e| e.to_string())?;

    run_aseprite(&["-b", file, "--script", script_path.to_str().unwrap()])?;

    let meta_content = fs::read_to_string(&json_path)
        .map_err(|e| format!("Failed to read export metadata: {}", e))?;

    let _ = fs::remove_file(&script_path);
    let _ = fs::remove_file(&json_path);

    let mut results = Vec::new();
    for line in meta_content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() == 4 {
            results.push(ExportResult {
                tag: parts[0].to_string(),
                frame_count: parts[1].parse().unwrap_or(1),
                width: parts[2].parse().unwrap_or(0),
                height: parts[3].parse().unwrap_or(0),
            });
        }
    }

    Ok(results)
}

fn get_all_frame_ranges(file: &str) -> Result<HashMap<String, u32>, String> {
    let temp_json = std::env::temp_dir().join("build_frame_meta.json");
    let temp_json_str = temp_json.to_str().unwrap();

    run_aseprite(&[
        "-b",
        file,
        "--list-tags",
        "--data",
        temp_json_str,
        "--format",
        "json-array",
    ])?;

    let json_content = fs::read_to_string(&temp_json)
        .map_err(|e| format!("Failed to read frame metadata: {}", e))?;
    let _ = fs::remove_file(&temp_json);

    let json: serde_json::Value = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse frame metadata: {}", e))?;

    let mut ranges = HashMap::new();
    if let Some(frame_tags) = json["meta"]["frameTags"].as_array() {
        for ft in frame_tags {
            let name = ft["name"].as_str().unwrap_or("");
            let from = ft["from"].as_u64().unwrap_or(0) as u32;
            ranges.insert(name.to_string(), from);
        }
    }

    Ok(ranges)
}

fn write_metadata(
    path: &str,
    source_file: &str,
    exports: &[ExportResult],
    frame_ranges: &HashMap<String, u32>,
) -> Result<(), String> {
    let source_mtime = fs::metadata(source_file)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let mut tags_json = String::from("{");
    for (i, export) in exports.iter().enumerate() {
        if i > 0 {
            tags_json.push_str(",");
        }
        let frame_from = frame_ranges.get(&export.tag).copied().unwrap_or(0);
        tags_json.push_str(&format!(
            r#""{}": {{"frame_from": {}, "frame_count": {}, "width": {}, "height": {}}}"#,
            export.tag, frame_from, export.frame_count, export.width, export.height
        ));
    }
    tags_json.push('}');

    let metadata = format!(
        r#"{{
  "source": "{}",
  "source_mtime": {},
  "tags": {}
}}"#,
        source_file.replace('\\', "/"),
        source_mtime,
        tags_json
    );

    fs::write(path, metadata).map_err(|e| e.to_string())
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}
