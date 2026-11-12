// Command-line front end: parses the flags, then previews or creates the storage.
use std::path::PathBuf;
use std::process::ExitCode;

use create_new_storage::{create, load_template, plan, sanitize_name, CreateOptions, Level, PlanNode};

const HELP: &str = "\
create-new-storage: build a new ANT Storage from a Markdown template.

Usage:
  create-new-storage <path> [options]

Arguments:
  <path>               Where the storage is created. With --name, the parent folder
                       that receives a new folder named after it.

Options:
  --name <name>        Storage folder name, cleaned to Pascal_Snake_Case
                       (\"my storage\" becomes My_Storage).
  --template <file>
                       Use a local template.md instead of downloading the official one.
  --yes                Allow creating inside a folder that already exists.
  --dry-run            Print the folders that would be created and touch nothing.
  --no-folder-manager  Do not download FolderManager into the storage.
  -h, --help           Show this help.
  -V, --version        Show the version.

Examples:
  create-new-storage ~ --name \"My Storage\"
  create-new-storage D:\\ --name Work --dry-run
  create-new-storage ./Store --template template.md --no-folder-manager --yes

What it creates:
  The folder tree from the template; each category also gets REC.<NAME>,
  RES.Resources, REV.<NAME> and SS.<NAME>; each subcategory and topic gets
  REV.<NAME>. The root gets ANT.RECORDINGS and
  ANT.SCREENSHOTS, .ant/ with template.md (a copy) and FM_Updater.bat, and
  FolderManager in the storage root. Up to 9 categories (100 to 900).
  If an essential step fails, only the folders made by this run are removed.

Full reference: https://ant.mikeworld.dev/en/advanced/cli/";

struct Args {
    opts: CreateOptions,
    dry_run: bool,
}

enum Parsed {
    Run(Args),
    Help,
    Version,
}

fn parse_args() -> Result<Parsed, String> {
    let mut args = std::env::args().skip(1);
    let (mut path, mut name, mut template, mut yes, mut folder_manager, mut dry_run) = (None, None, None, false, true, false);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--name" => name = Some(args.next().ok_or("--name needs a value")?),
            "--template" | "--local-database" => template = Some(args.next().ok_or("--template needs a value")?),
            "--yes" => yes = true,
            "--dry-run" => dry_run = true,
            "--no-folder-manager" => folder_manager = false,
            "-h" | "--help" => return Ok(Parsed::Help),
            "-V" | "--version" => return Ok(Parsed::Version),
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
    let template = match template {
        Some(file) => Some(std::fs::read(&file).map_err(|e| format!("Could not read {file}: {e}"))?),
        None => None,
    };
    Ok(Parsed::Run(Args { opts: CreateOptions { root, template, allow_existing: yes, download: true, folder_manager }, dry_run }))
}

fn print_plan(nodes: &[PlanNode], depth: usize) {
    for node in nodes {
        println!("{}{}  [{}]", "  ".repeat(depth + 1), node.name, node.kind);
        print_plan(&node.children, depth + 1);
    }
}

fn dry_run(opts: &CreateOptions) -> Result<(), String> {
    let nodes = plan(&load_template(opts)?)?;
    println!("Dry run: nothing is written.\n{}", opts.root.display());
    print_plan(&nodes, 0);
    if opts.root.exists() && !opts.allow_existing {
        println!("\n{} already exists; a real run needs --yes", opts.root.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(Parsed::Run(args)) => args,
        Ok(Parsed::Help) => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Ok(Parsed::Version) => {
            println!("create-new-storage {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("{e}\nRun create-new-storage --help for usage.");
            return ExitCode::FAILURE;
        }
    };
    let opts = &args.opts;
    if args.dry_run {
        return match dry_run(opts) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Dry run failed: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if opts.root.exists() && !opts.allow_existing {
        eprintln!("{} already exists; pass --yes to create inside it", opts.root.display());
        return ExitCode::FAILURE;
    }
    let mut print = |level: Level, msg: &str| match level {
        Level::Step => println!("\n{msg}"),
        Level::Error | Level::Warn => eprintln!("{msg}"),
        Level::Info | Level::Ok => println!("{msg}"),
    };
    match create(opts, &mut print) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Storage not created: {e}");
            ExitCode::FAILURE
        }
    }
}
