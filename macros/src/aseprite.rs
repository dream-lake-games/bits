use anyhow::{Context, Result};
use std::{ffi::OsStr, process::Command};

const ASEPRITE_BIN: &str = "/Applications/Aseprite.app/Contents/MacOS/aseprite";

fn run_aseprite_cmd<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if !std::path::Path::new(ASEPRITE_BIN).exists() {
        anyhow::bail!("Aseprite not found at: {}", ASEPRITE_BIN);
    }

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

fn list_layers(file: &str) -> Result<Vec<String>> {
    let output = run_aseprite_cmd(["-b", "--list-layers", file])?;
    Ok(output.lines().map(|s| s.to_string()).collect())
}

pub fn list_tags(file: &str) -> Result<Vec<String>> {
    let output = run_aseprite_cmd(["-b", "--list-tags", file])?;
    Ok(output.split_whitespace().map(|s| s.to_string()).collect())
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

pub struct ExportBuilder {
    file: String,
    tag: String,
    exclude_prefixes: Vec<String>,
}

impl ExportBuilder {
    pub fn new(file: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            tag: tag.into(),
            exclude_prefixes: Vec::new(),
        }
    }

    pub fn exclude_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.exclude_prefixes.push(prefix.into());
        self
    }

    fn get_layers_to_ignore(&self) -> Result<Vec<String>> {
        let all_layers = list_layers(&self.file)?;
        Ok(all_layers
            .into_iter()
            .filter(|layer| {
                self.exclude_prefixes
                    .iter()
                    .any(|prefix| layer.starts_with(prefix))
            })
            .collect())
    }

    pub fn export_to_file(self, output_path: &str) -> Result<()> {
        let layers_to_ignore = self.get_layers_to_ignore()?;

        let mut args = vec!["-b".to_string()];
        args.push(self.file.clone());
        args.push("--tag".to_string());
        args.push(self.tag.clone());

        for layer in layers_to_ignore {
            args.push("--ignore-layer".to_string());
            args.push(layer);
        }

        args.push("--frame-range".to_string());
        args.push("0,0".to_string());
        args.push("--save-as".to_string());
        args.push(output_path.to_string());

        run_aseprite_cmd(args.iter().map(|s| s.as_str()))?;

        if !std::path::Path::new(output_path).exists() {
            anyhow::bail!(
                "Aseprite export did not create output file.\nArgs: {:?}",
                args
            );
        }

        Ok(())
    }

    pub fn export_sprite_sheet(self, output_path: &str) -> Result<AnimExportInfo> {
        let layers_to_ignore = self.get_layers_to_ignore()?;

        let (frame_from, frame_to) = get_tag_frame_range(&self.file, &self.tag)?;

        let temp_dir = std::env::temp_dir();
        let temp_json = temp_dir.join(format!(
            "anim_sheet_{}_{}.json",
            self.file.replace(['/', '\\', '.'], "_"),
            self.tag
        ));

        let mut args = vec!["-b".to_string()];
        args.push("--frame-range".to_string());
        args.push(format!("{},{}", frame_from, frame_to));

        for layer in &layers_to_ignore {
            args.push("--ignore-layer".to_string());
            args.push(layer.clone());
        }

        args.push(self.file.clone());

        args.push("--sheet".to_string());
        args.push(output_path.to_string());
        args.push("--sheet-type".to_string());
        args.push("horizontal".to_string());
        args.push("--data".to_string());
        args.push(temp_json.to_str().unwrap().to_string());
        args.push("--format".to_string());
        args.push("json-array".to_string());

        run_aseprite_cmd(args.iter().map(|s| s.as_str()))?;

        if !std::path::Path::new(output_path).exists() {
            anyhow::bail!(
                "Aseprite sprite sheet export did not create output file.\nArgs: {:?}",
                args
            );
        }

        let json_content = std::fs::read_to_string(&temp_json)
            .with_context(|| format!("Failed to read temp json: {}", temp_json.display()))?;
        let _ = std::fs::remove_file(&temp_json);

        let json: serde_json::Value = serde_json::from_str(&json_content)
            .with_context(|| "Failed to parse aseprite JSON output")?;

        let frames = json["frames"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No frames array in JSON"))?;

        let frame_count = frames.len();
        if frame_count == 0 {
            anyhow::bail!("Tag '{}' has no frames in sprite sheet", self.tag);
        }

        let first_frame = &frames[0];
        let frame_width = first_frame["sourceSize"]["w"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing frame width"))?
            as u32;
        let frame_height = first_frame["sourceSize"]["h"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing frame height"))?
            as u32;

        Ok(AnimExportInfo {
            frame_count,
            frame_width,
            frame_height,
        })
    }
}

#[derive(Debug)]
pub struct AnimExportInfo {
    pub frame_count: usize,
    pub frame_width: u32,
    pub frame_height: u32,
}

fn get_tag_frame_range(file: &str, tag: &str) -> Result<(u32, u32)> {
    let temp_dir = std::env::temp_dir();
    let temp_json = temp_dir.join(format!(
        "tag_meta_{}.json",
        file.replace(['/', '\\', '.'], "_")
    ));

    let args = vec![
        "-b",
        file,
        "--list-tags",
        "--data",
        temp_json.to_str().unwrap(),
        "--format",
        "json-array",
    ];

    run_aseprite_cmd(args)?;

    let json_content = std::fs::read_to_string(&temp_json)
        .with_context(|| format!("Failed to read temp json: {}", temp_json.display()))?;
    let _ = std::fs::remove_file(&temp_json);

    let json: serde_json::Value = serde_json::from_str(&json_content)
        .with_context(|| "Failed to parse aseprite JSON output")?;

    let frame_tags = json["meta"]["frameTags"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No frameTags in metadata"))?;

    for frame_tag in frame_tags {
        let name = frame_tag["name"].as_str().unwrap_or("");
        if name == tag {
            let from = frame_tag["from"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing 'from' in tag"))?
                as u32;
            let to = frame_tag["to"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing 'to' in tag"))? as u32;
            return Ok((from, to));
        }
    }

    anyhow::bail!("Tag '{}' not found in frameTags metadata", tag)
}
