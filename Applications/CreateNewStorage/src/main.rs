// Command-line front end: create-new-storage <path> [--name <n>] [--tree <file>] [--yes] [--no-download].
use std::path::PathBuf;
use std::process::ExitCode;

use create_new_storage::{create, sanitize_name, CreateOptions, Level};

const USAGE: &str = "Usage: create-new-storage <path> [--name <name>] [--tree <Tree.yaml>] [--yes] [--no-download]";

fn parse_args() -> Result<(CreateOptions, bool), String> {
    let mut args = std::env::args().skip(1);
    let (mut path, mut name, mut tree, mut yes, mut download) = (None, None, None, false, true);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--name" => name = Some(args.next().ok_or("--name needs a value")?),
            "--tree" => tree = Some(args.next().ok_or("--tree needs a value")?),
            "--yes" => yes = true,
            "--no-download" => download = false,
            "-h" | "--help" => return Err(String::new()),
            a if a.starts_with('-') => return Err(format!("Unknown option {a}")),
            _ if path.is_some() => return Err("Only one path is allowed".to_string()),
            _ => path = Some(PathBuf::from(arg)),
        }
    }
    let mut root = path.ok_or("Missing <path>")?;
    // With --name, <path> is the parent and the sanitized name becomes the storage folder.
    if let Some(raw) = name {
        let clean = sanitize_name(&raw);
        if clean.is_empty() {
            return Err(format!("{raw:?} leaves no usable folder name"));
        }
        root = root.join(clean);
    }
    let tree_yaml = match tree {
        Some(file) => Some(std::fs::read_to_string(&file).map_err(|e| format!("Could not read {file}: {e}"))?),
        None => None,
    };
    Ok((CreateOptions { root, tree_yaml, allow_existing: yes, download }, yes))
}

fn main() -> ExitCode {
    let (opts, yes) = match parse_args() {
        Ok(parsed) => parsed,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}");
            }
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    if opts.root.exists() && !yes {
        eprintln!("{} already exists; pass --yes to create inside it", opts.root.display());
        return ExitCode::FAILURE;
    }
    let mut print = |level: Level, msg: &str| match level {
        Level::Step => println!("\n{msg}"),
        Level::Error | Level::Warn => eprintln!("{msg}"),
        Level::Info | Level::Ok => println!("{msg}"),
    };
    match create(&opts, &mut print) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Storage not created: {e}");
            ExitCode::FAILURE
        }
    }
}
