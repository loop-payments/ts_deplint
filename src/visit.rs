use crate::{
    disallowed, files, rules, ts_reader,
    violations::{DisallowedImportViolation, Violation},
};
use std::{error::Error, path::Component, path::Path, path::PathBuf};

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
            let normalized_import = normalize_import(&import, root, current);
            for disallowed_import in disallowed_imports {
                if normalized_import.starts_with(disallowed_import) {
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

fn normalize_import(import: &str, root: &Path, current: &Path) -> String {
    if import.starts_with(".") {
        let full_path = current.join(Path::new(&import));
        let normalized_path = normalize_path(&full_path);
        return normalized_path
            .strip_prefix(root)
            .expect("Failed to strip prefix")
            .to_str()
            .expect("Failed to convert path to string")
            .to_string();
    }

    return import.to_string();
}

#[test]
fn test_normalize_import() {
    assert_eq!(
        normalize_import(
            "../other-code/test-file",
            Path::new("/Home/code/lib/system"),
            Path::new("/Home/code/lib/system/src/entity")
        ),
        "src/other-code/test-file"
    );
    assert_eq!(
        normalize_import(
            "./sub-dir/test-file",
            Path::new("/Home/code/lib/system"),
            Path::new("/Home/code/lib/system/src/entity")
        ),
        "src/entity/sub-dir/test-file"
    );
    assert_eq!(
        normalize_import(
            "src/entity/sub-dir/test-file",
            Path::new("/Home/code/lib/system"),
            Path::new("/Home/code/lib/system/src/entity")
        ),
        "src/entity/sub-dir/test-file"
    );
}

// Copied from https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}
