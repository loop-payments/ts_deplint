use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;

use ts_deplint::{
    find_package_json_directory, list_violations, pretty_print_violations,
    update_diagrams_recursively, update_readme_with_diagram, Violation, RULES_FILE_NAME,
};

#[derive(Parser)]
#[clap(name = "ts_depslint")]
/// ts_deplint is a tool for linting TypeScript projects for disallowed imports.
struct Opt {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Lint(LintCommand),
    Diagram(DiagramCommand),
    Fix(FixCommand),
    Format(FormatCommand),
}

#[derive(Parser)]
#[clap(rename_all = "camel_case")]
/// Lint the passed-in paths for disallowed imports.
struct LintCommand {
    /// Paths can be either directories or files.
    paths: Vec<String>,
}

#[derive(Parser)]
#[clap(rename_all = "camel_case")]
/// Update the README.md file in the passed-in paths with a diagram of the disallowed imports.
struct DiagramCommand {
    /// Paths can be either directories or files.
    paths: Vec<String>,

    #[arg(short, long, default_value_t = false)]
    show_circular_dependencies: bool,
}

#[derive(Parser)]
#[clap(rename_all = "camel_case")]
/// Fix the disallowed imports in the passed-in paths by adding allow rules.
struct FixCommand {
    /// Paths can be either directories or files.
    paths: Vec<String>,
}

#[derive(Parser)]
#[clap(rename_all = "camel_case")]
/// Format the rules files in the passed-in paths.
struct FormatCommand {
    /// Paths can be either directories or files.
    paths: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();

    match opt.command {
        Commands::Lint(command) => run_lint_command(command),
        Commands::Diagram(command) => run_diagram_command(command),
        Commands::Fix(command) => run_fix_command(command),
        Commands::Format(command) => run_format_command(command),
    }
}

fn run_lint_command(command: LintCommand) -> Result<(), Box<dyn Error>> {
    let Ok(sample_path) = fs::canonicalize(Path::new(&command.paths[0])) else {
        return Err(format!("Target path '{}' does not exist.", &command.paths[0]).into());
    };
    let root = find_package_json_directory(&sample_path)
        .ok_or("No package.json found in any parent directory.")?;

    let mut all_violations: HashSet<Violation> = HashSet::new();
    for path in command.paths.iter() {
        let Ok(target) = fs::canonicalize(Path::new(path)) else {
            return Err(format!("Target path '{}' does not exist.", path).into());
        };
        let violations = list_violations(&root, &target, false)?;
        all_violations.extend(violations);
    }

    if all_violations.len() > 0 {
        let count = all_violations.len();
        pretty_print_violations(all_violations);
        return Err(format!("{} violations.", count).into());
    }

    Ok(())
}

fn run_diagram_command(command: DiagramCommand) -> Result<(), Box<dyn Error>> {
    for path in command.paths.iter() {
        let Ok(target) = fs::canonicalize(Path::new(path)) else {
            return Err(format!("Target path '{}' does not exist.", path).into());
        };
        if target.ends_with(RULES_FILE_NAME) {
            let readme_path = target.parent().unwrap().join("README.md");
            update_readme_with_diagram(&target, &readme_path, command.show_circular_dependencies)?;
        } else if target.is_dir() {
            update_diagrams_recursively(&target, command.show_circular_dependencies)?;
        } else {
            return Err(format!("Target path '{}' is not a rules file or directory.", path).into());
        }
    }

    Ok(())
}

fn run_fix_command(command: FixCommand) -> Result<(), Box<dyn Error>> {
    let Ok(sample_path) = fs::canonicalize(Path::new(&command.paths[0])) else {
        return Err(format!("Target path '{}' does not exist.", &command.paths[0]).into());
    };
    let root = find_package_json_directory(&sample_path)
        .ok_or("No package.json found in any parent directory.")?;

    let mut i = 0;
    for path in command.paths.iter() {
        loop {
            let Ok(target) = fs::canonicalize(Path::new(path)) else {
                return Err(format!("Target path '{}' does not exist.", path).into());
            };
            let violations = list_violations(&root, &target, true)?;
            if violations.len() == 0 {
                break;
            }
            for violation in violations {
                match violation {
                    Violation::DisallowedImportViolation(violation) => {
                        ts_deplint::fix_violation(&root, &violation)?;
                    }
                    Violation::ReferenceToNonexistentDirectory(issue) => {
                        ts_deplint::remove_reference_to_nonexistent_directory(&root, &issue)?;
                    }
                }
            }
            i += 1;
            if i > 500 {
                return Err("Looped 500 times. Something is wrong.".into());
            }
        }
    }

    Ok(())
}

fn run_format_command(command: FormatCommand) -> Result<(), Box<dyn Error>> {
    for path in command.paths.iter() {
        let Ok(target) = fs::canonicalize(Path::new(path)) else {
            return Err(format!("Target path '{}' does not exist.", path).into());
        };
        if target.ends_with(RULES_FILE_NAME) {
            ts_deplint::format_rules_file(&target)?;
        } else {
            ts_deplint::format_rules_files_recursively(&target)?;
        }
    }

    Ok(())
}
