use anyhow::{Context, Result};
use fs2::FileExt;
use std::{fs::File, path::Path, process::Command, time::SystemTime};

const ASEPRITE_BIN: &str = "/Users/mork/Library/Application Support/Steam/steamapps/common/Aseprite/Aseprite.app/Contents/MacOS/aseprite";

// ============================================================================
// Aseprite Commands (with caching)
// ============================================================================

fn run_aseprite_cmd(args: &[&str]) -> Result<String> {
    if !Path::new(ASEPRITE_BIN).exists() {
        anyhow::bail!("Aseprite not found at: {}", ASEPRITE_BIN);
    }

    let lock = File::create(std::env::temp_dir().join("aseprite_macro.lock"))?;
    lock.lock_exclusive()?;

    let output = Command::new(ASEPRITE_BIN)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute Aseprite at: {}", ASEPRITE_BIN))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Aseprite command failed with exit code {:?}:\n{}",
            output.status.code(),
            stderr
        );
    }

    let stdout =
        std::str::from_utf8(&output.stdout).context("Aseprite output was not valid UTF-8")?;
    Ok(stdout.into())
}

fn get_mtime(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn is_cache_valid(source_file: &str, output_files: &[String]) -> bool {
    let Some(source_mtime) = get_mtime(source_file) else {
        return false;
    };
    for output in output_files {
        let Some(output_mtime) = get_mtime(output) else {
            return false;
        };
        if source_mtime > output_mtime {
            return false;
        }
    }
    true
}

pub fn list_tags(file: &str) -> Result<Vec<String>> {
    let cache_path = std::env::temp_dir().join(format!(
        "aseprite_tags_{}.txt",
        file.replace(['/', '\\', '.', ' '], "_")
    ));

    if let (Some(src_mtime), Ok(cache_content)) =
        (get_mtime(file), std::fs::read_to_string(&cache_path))
    {
        if let Some(cache_mtime) = get_mtime(cache_path.to_str().unwrap()) {
            if cache_mtime > src_mtime {
                return Ok(cache_content.lines().map(|s| s.to_string()).collect());
            }
        }
    }

    let output = run_aseprite_cmd(&["-b", "--list-tags", file])?;
    let tags: Vec<String> = output.split_whitespace().map(|s| s.to_string()).collect();
    let _ = std::fs::write(&cache_path, tags.join("\n"));
    Ok(tags)
}

fn list_layers(file: &str) -> Result<Vec<String>> {
    let cache_path = std::env::temp_dir().join(format!(
        "aseprite_layers_{}.txt",
        file.replace(['/', '\\', '.', ' '], "_")
    ));

    if let (Some(src_mtime), Ok(cache_content)) =
        (get_mtime(file), std::fs::read_to_string(&cache_path))
    {
        if let Some(cache_mtime) = get_mtime(cache_path.to_str().unwrap()) {
            if cache_mtime > src_mtime {
                return Ok(cache_content.lines().map(|s| s.to_string()).collect());
            }
        }
    }

    let output = run_aseprite_cmd(&["-b", "--list-layers", file])?;
    let layers: Vec<String> = output.lines().map(|s| s.to_string()).collect();
    let _ = std::fs::write(&cache_path, layers.join("\n"));
    Ok(layers)
}

pub fn validate_tag(file: &str, tag: &str) -> Result<()> {
    let tags = list_tags(file)?;
    if !tags.iter().any(|t| t == tag) {
        anyhow::bail!(
            "Tag '{}' not found in '{}'. Available tags: {:?}",
            tag,
            file,
            tags
        );
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct AnimExportInfo {
    pub frame_count: usize,
    pub frame_width: u32,
    pub frame_height: u32,
}

#[derive(Debug)]
pub struct BatchExportResult {
    pub tag: String,
    pub output_path: String,
    pub info: AnimExportInfo,
}

pub fn batch_export_sprite_sheets(
    file: &str,
    tags: &[String],
    output_dir: &str,
    exclude_prefixes: &[String],
) -> Result<Vec<BatchExportResult>> {
    std::fs::create_dir_all(output_dir)?;

    let output_files: Vec<String> = tags
        .iter()
        .map(|tag| format!("{}/{}.png", output_dir, tag))
        .collect();

    let cache_json_path = format!("{}/_cache.json", output_dir);

    if is_cache_valid(file, &output_files) && Path::new(&cache_json_path).exists() {
        let cache_content = std::fs::read_to_string(&cache_json_path)?;
        let cached: Vec<BatchExportResult> = parse_cache_json(&cache_content, tags, output_dir)?;
        if cached.len() == tags.len() {
            return Ok(cached);
        }
    }

    let layers_to_ignore: Vec<String> = if !exclude_prefixes.is_empty() {
        list_layers(file)?
            .into_iter()
            .filter(|layer| exclude_prefixes.iter().any(|p| layer.starts_with(p)))
            .collect()
    } else {
        vec![]
    };

    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("batch_export.lua");
    let json_path = temp_dir.join("batch_export_meta.json");

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

    std::fs::write(&script_path, &lua_script)?;
    run_aseprite_cmd(&["-b", file, "--script", script_path.to_str().unwrap()])?;

    let meta_content = std::fs::read_to_string(&json_path)
        .with_context(|| "Failed to read batch export metadata")?;

    let _ = std::fs::remove_file(&script_path);
    let _ = std::fs::remove_file(&json_path);

    let mut results = Vec::new();
    for line in meta_content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() == 4 {
            let tag = parts[0].to_string();
            let frame_count: usize = parts[1].parse().unwrap_or(1);
            let frame_width: u32 = parts[2].parse().unwrap_or(0);
            let frame_height: u32 = parts[3].parse().unwrap_or(0);
            let output_path = format!("{}/{}.png", output_dir, tag);
            results.push(BatchExportResult {
                tag,
                output_path,
                info: AnimExportInfo {
                    frame_count,
                    frame_width,
                    frame_height,
                },
            });
        }
    }

    let cache_content = results
        .iter()
        .map(|r| {
            format!(
                "{},{},{},{}",
                r.tag, r.info.frame_count, r.info.frame_width, r.info.frame_height
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(&cache_json_path, cache_content);

    Ok(results)
}

fn parse_cache_json(
    content: &str,
    tags: &[String],
    output_dir: &str,
) -> Result<Vec<BatchExportResult>> {
    let mut results = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() == 4 {
            let tag = parts[0].to_string();
            if tags.contains(&tag) {
                let frame_count: usize = parts[1].parse().unwrap_or(1);
                let frame_width: u32 = parts[2].parse().unwrap_or(0);
                let frame_height: u32 = parts[3].parse().unwrap_or(0);
                let output_path = format!("{}/{}.png", output_dir, tag);
                results.push(BatchExportResult {
                    tag,
                    output_path,
                    info: AnimExportInfo {
                        frame_count,
                        frame_width,
                        frame_height,
                    },
                });
            }
        }
    }
    Ok(results)
}

pub fn export_single_frame(
    file: &str,
    tag: &str,
    output_path: &str,
    exclude_prefixes: &[String],
) -> Result<()> {
    if is_cache_valid(file, &[output_path.to_string()]) {
        return Ok(());
    }

    let layers_to_ignore: Vec<String> = if !exclude_prefixes.is_empty() {
        list_layers(file)?
            .into_iter()
            .filter(|layer| exclude_prefixes.iter().any(|p| layer.starts_with(p)))
            .collect()
    } else {
        vec![]
    };

    let frame_from = get_tag_first_frame_cached(file, tag)?;

    let mut args: Vec<String> = vec!["-b".into(), file.into()];
    for layer in &layers_to_ignore {
        args.push("--ignore-layer".into());
        args.push(layer.clone());
    }
    args.push("--frame-range".into());
    args.push(format!("{},{}", frame_from, frame_from));
    args.push("--save-as".into());
    args.push(output_path.into());

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_aseprite_cmd(&args_refs)?;

    Ok(())
}

fn get_tag_first_frame_cached(file: &str, tag: &str) -> Result<u32> {
    let cache_path = std::env::temp_dir().join(format!(
        "aseprite_frames_{}.txt",
        file.replace(['/', '\\', '.', ' '], "_")
    ));

    if let (Some(src_mtime), Ok(cache_content)) =
        (get_mtime(file), std::fs::read_to_string(&cache_path))
    {
        if let Some(cache_mtime) = get_mtime(cache_path.to_str().unwrap()) {
            if cache_mtime > src_mtime {
                for line in cache_content.lines() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() == 2 && parts[0] == tag {
                        if let Ok(frame) = parts[1].parse() {
                            return Ok(frame);
                        }
                    }
                }
            }
        }
    }

    let temp_dir = std::env::temp_dir();
    let temp_json = temp_dir.join("frame_meta.json");
    let temp_json_str = temp_json.to_str().unwrap();

    run_aseprite_cmd(&[
        "-b",
        file,
        "--list-tags",
        "--data",
        temp_json_str,
        "--format",
        "json-array",
    ])?;

    let json_content = std::fs::read_to_string(&temp_json)?;
    let _ = std::fs::remove_file(&temp_json);
    let json: serde_json::Value = serde_json::from_str(&json_content)?;

    let frame_tags = json["meta"]["frameTags"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No frameTags"))?;

    let mut cache_lines = Vec::new();
    let mut result = 0u32;
    for ft in frame_tags {
        let name = ft["name"].as_str().unwrap_or("");
        let from = ft["from"].as_u64().unwrap_or(0) as u32;
        cache_lines.push(format!("{},{}", name, from));
        if name == tag {
            result = from;
        }
    }

    let _ = std::fs::write(&cache_path, cache_lines.join("\n"));
    Ok(result)
}
