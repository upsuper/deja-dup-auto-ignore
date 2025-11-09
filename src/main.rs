use anyhow::{Context, Result};
use clap::Parser;
use log::{info, warn};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

mod stack_vec;
mod traverse;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Dry run mode - list directories without creating .deja-dup-ignore files
    #[arg(short = 'n', long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    let (include_paths, exclude_paths) = read_deja_dup_config()?;
    info!("Include paths: {:?}", include_paths);
    info!("Exclude paths: {:?}", exclude_paths);

    let exclude_paths = exclude_paths
        .iter()
        .map(|p| p.as_path())
        .collect::<Vec<_>>();
    let mut cb = if args.dry_run {
        (|path: &Path| println!("{}", path.display())) as fn(&Path)
    } else {
        (|path: &Path| {
            let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
                return;
            };
            let file_to_create = match file_name {
                "node_modules" | "venv" | ".venv" | ".gradle" | "target" | "build" | "out"
                | "dist" => ".deja-dup-ignore",
                str if str.contains("cache") => "CACHEDIR.TAG",
                _ => return,
            };
            match File::create(path.join(file_to_create)) {
                Ok(_) => info!("Created {file_to_create} in {}", path.display()),
                Err(e) => warn!(
                    "Failed to create {file_to_create} in {}: {e}",
                    path.display()
                ),
            }
        }) as fn(&Path)
    };
    for include_path in &include_paths {
        let canonical_root = fs::canonicalize(include_path).with_context(|| {
            format!(
                "Failed to canonicalize root path: {}",
                include_path.display()
            )
        })?;
        traverse::find_directory_to_ignore(&canonical_root, &exclude_paths, &mut cb)?;
    }

    info!("Done!");
    Ok(())
}

fn read_deja_dup_config() -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let include_output = Command::new("dconf")
        .args(["read", "/org/gnome/deja-dup/include-list"])
        .output()
        .context("Failed to read include-list from dconf")?;

    let exclude_output = Command::new("dconf")
        .args(["read", "/org/gnome/deja-dup/exclude-list"])
        .output()
        .context("Failed to read exclude-list from dconf")?;

    let include_paths = parse_dconf_list(&include_output.stdout)?;
    let exclude_paths = parse_dconf_list(&exclude_output.stdout)?;

    Ok((include_paths, exclude_paths))
}

fn parse_dconf_list(output: &[u8]) -> Result<Vec<PathBuf>> {
    let output_str = String::from_utf8_lossy(output);
    let trimmed = output_str.trim();

    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();

    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inner = &trimmed[1..trimmed.len() - 1];
        for item in inner.split(',') {
            let item = item.trim();
            if item.starts_with('\'') && item.ends_with('\'') {
                let path_str = &item[1..item.len() - 1];
                let expanded = shellexpand::tilde(path_str);
                paths.push(PathBuf::from(expanded.as_ref()));
            }
        }
    }

    Ok(paths)
}
