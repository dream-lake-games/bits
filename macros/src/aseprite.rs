use anyhow::{Context, Result};
use std::{ffi::OsStr, process::Command};

const ASEPRITE_BIN: &str = "/Applications/Aseprite.app/Contents/MacOS/aseprite";

fn run_aseprite_cmd<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(ASEPRITE_BIN)
        .args(args)
        .output()
        .context("Failed to execute Aseprite command")?;

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

fn list_tags(file: &str) -> Result<Vec<String>> {
    let output = run_aseprite_cmd(["-b", "--list-tags", file])?;
    Ok(output
        .split_whitespace()
        .into_iter()
        .map(|s| s.to_string())
        .collect())
}

fn list_layers(file: &str) -> Result<Vec<String>> {
    let output = run_aseprite_cmd(["-b", "--list-layers", file])?;
    Ok(output.lines().map(|s| s.to_string()).collect())
}

#[allow(dead_code)]
pub(crate) fn check_if_file_has_tag(file: &str, tag: &str) -> Result<bool> {
    Ok(list_tags(file)?.iter().any(|check_tag| check_tag == tag))
}

pub(crate) struct ExportBuilder {
    file: String,
    tag: String,
    include_prefixes: Vec<String>,
    exclude_prefixes: Vec<String>,
}

impl ExportBuilder {
    pub(crate) fn new(file: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            tag: tag.into(),
            include_prefixes: Vec::new(),
            exclude_prefixes: Vec::new(),
        }
    }

    pub(crate) fn get_generated_folder(file: &str) -> Result<String> {
        if !file.ends_with(".aseprite") {
            anyhow::bail!("File must end with .aseprite: {}", file);
        }

        let parts: Vec<&str> = file.split('/').collect();
        let assets_index = parts
            .iter()
            .rposition(|&part| part == "assets")
            .ok_or_else(|| {
                anyhow::anyhow!("File path must contain 'assets' component: {}", file)
            })?;

        let mut result_parts = parts[..=assets_index].to_vec();
        result_parts.push("_generated");
        result_parts.push("aseprite");
        result_parts.extend_from_slice(&parts[assets_index + 1..]);

        Ok(format!("{}/", result_parts.join("/")))
    }

    pub(crate) fn get_data_file(file: &str, tag: &str) -> Result<String> {
        let folder = Self::get_generated_folder(file)?;
        Ok(format!("{}{}_data.json", folder, tag))
    }

    pub(crate) fn get_sprite_file(file: &str, tag: &str) -> Result<String> {
        let folder = Self::get_generated_folder(file)?;
        Ok(format!("{}{}_sprite.png", folder, tag))
    }

    pub(crate) fn include_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.include_prefixes.push(prefix.into());
        self
    }

    pub(crate) fn exclude_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.exclude_prefixes.push(prefix.into());
        self
    }

    pub(crate) fn export(self) -> Result<()> {
        let output_data = Self::get_data_file(&self.file, &self.tag)?;
        let output_sheet = Self::get_sprite_file(&self.file, &self.tag)?;

        let all_layers = list_layers(&self.file)?;
        let layers_to_ignore: Vec<String> = all_layers
            .iter()
            .filter(|layer| {
                let should_include = if self.include_prefixes.is_empty() {
                    true
                } else {
                    self.include_prefixes
                        .iter()
                        .any(|prefix| layer.starts_with(prefix))
                };

                let should_exclude = self
                    .exclude_prefixes
                    .iter()
                    .any(|prefix| layer.starts_with(prefix));

                !should_include || should_exclude
            })
            .cloned()
            .collect();

        let mut args = vec!["-b".to_string()];

        args.push("--tag".to_string());
        args.push(self.tag.clone());

        for layer in layers_to_ignore {
            args.push("--ignore-layer".to_string());
            args.push(layer);
        }

        args.push("--list-layers".to_string());
        args.push("--sheet".to_string());
        args.push(output_sheet);
        args.push("--data".to_string());
        args.push(output_data);
        args.push("--format".to_string());
        args.push("json-array".to_string());

        args.push(self.file.clone());

        run_aseprite_cmd(args)?;
        Ok(())
    }
}
