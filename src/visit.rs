use crate::{disallowed, files, rules, ts_reader, violations::Violation};
use std::{
    error::Error,
    path::{Path, PathBuf},
};

pub fn visit_path(
    violations: &mut Vec<Violation>,
    root: &Path,
    disallowed_imports: &Vec<String>,
    current: &Path,
    abort_on_violation: bool,
) -> Result<(), Box<dyn Error>> {
    let files_and_directories = files::list_files_and_directories(current)?;

    check_files_for_disallowed_imports(
        violations,
        root,
        disallowed_imports,
        &files_and_directories.files,
        abort_on_violation,
    )?;
    if abort_on_violation && violations.len() > 0 {
        return Ok(());
    }

    visit_directories(
        violations,
        root,
        disallowed_imports,
        &current,
        &files_and_directories.directories,
        abort_on_violation,
    )?;

    Ok(())
}

fn check_files_for_disallowed_imports(
    violations: &mut Vec<Violation>,
    root: &Path,
    disallowed_imports: &Vec<String>,
    files: &[PathBuf],
    abort_on_violation: bool,
) -> Result<(), Box<dyn Error>> {
    for full_path in files {
        if full_path.extension().unwrap() != "ts" {
            continue;
        }
        let relative_path_result = full_path.strip_prefix(root);
        if relative_path_result.is_err() {
            println!("Failed to strip {:?} from {:?}", root, full_path);
        }
        let relative_path = relative_path_result.unwrap();
        let imports = ts_reader::read_ts_imports(&full_path)?;
        for import in imports {
            for disallowed_import in disallowed_imports {
                if import.starts_with(disallowed_import) {
                    let violation = Violation {
                        file_path: relative_path.to_str().expect("").to_string(),
                        disallowed_import: disallowed_import.clone(),
                    };
                    violations.push(violation);
                    if abort_on_violation {
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

fn visit_directories(
    violations: &mut Vec<Violation>,
    root: &Path,
    disallowed_imports: &Vec<String>,
    current: &Path,
    directories: &[PathBuf],
    abort_on_violation: bool,
) -> Result<(), Box<dyn Error>> {
    let current_rules = rules::get_dir_rules(current);
    for child in directories {
        let dir_disallowed_imports = disallowed::get_child_disallowed_imports(
            root,
            disallowed_imports,
            &current_rules,
            child,
        );
        let next = current.join(child);
        visit_path(
            violations,
            root,
            &dir_disallowed_imports,
            &next,
            abort_on_violation,
        )?;
        if abort_on_violation && violations.len() > 0 {
            return Ok(());
        }
    }

    Ok(())
}
