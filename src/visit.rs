use crate::{
    disallowed, files, rules, ts_reader,
    violations::{DisallowedImportViolation, Violation},
};
use std::{
    error::Error,
    fs::canonicalize,
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
        &current,
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
    current: &Path,
    files: &Vec<String>,
    abort_on_violation: bool,
) -> Result<(), Box<dyn Error>> {
    for file in files {
        if !file.ends_with(".ts") {
            continue;
        }

        let full_path = current.join(file);
        let relative_path = full_path.strip_prefix(root)?;

        let imports = ts_reader::read_ts_imports(&full_path)?;
        for import in imports {
            let canonicalized_import_path = canonicalize_import_path(&import, root, current)?;
            for disallowed_import in disallowed_imports {
                if canonicalized_import_path.starts_with(disallowed_import) {
                    let violation = DisallowedImportViolation {
                        file_path: relative_path.to_str().expect("").to_string(),
                        disallowed_import: disallowed_import.clone(),
                        full_disallowed_import: import.clone(),
                    };
                    violations.push(Violation::DisallowedImportViolation(violation));
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
    directories: &Vec<String>,
    abort_on_violation: bool,
) -> Result<(), Box<dyn Error>> {
    let (current_rules, rules_file_violations) = rules::get_dir_rules_if_exists(root, current);
    violations.extend(
        rules_file_violations
            .into_iter()
            .map(|issue| Violation::ReferenceToNonexistentDirectory(issue)),
    );
    for child in directories {
        let dir_disallowed_imports = disallowed::get_child_disallowed_imports(
            root,
            current,
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

fn canonicalize_import_path(
    import: &str,
    root_directory: &Path,
    current_directory: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    if import.trim().is_empty() {
        // Edge case handling. An empty string would lead to us
        // resolving the root directory, and stripping the
        // root_directory prefix would fail.
        return Ok(PathBuf::from(import));
    }

    let import_path = Path::new(&import);
    let file_name = import_path.file_name().unwrap_or_default();

    let fully_qualified_path: PathBuf;
    if import.starts_with(".") {
        // Relative imports are relative to the current directory of the file.
        fully_qualified_path = current_directory.join(import_path);
    } else {
        // Absolute imports are relative to the root directory of the project.
        fully_qualified_path = root_directory.join(import_path);
    }

    // We expect there to always be a parent directory since we
    // append the import path to a directory.
    let directory_path = fully_qualified_path
        .parent()
        .expect("Unable to get parent directory");

    let Ok(canonicalized_directory_path) = canonicalize(directory_path) else {
        // If the path doesn't exist, we assume it's an import from a third-party library
        // and return the original import path as is.
        return Ok(import_path.to_path_buf());
    };

    let path_from_root = canonicalized_directory_path
        .join(file_name)
        .strip_prefix(root_directory)?
        .to_path_buf();

    Ok(path_from_root)
}
