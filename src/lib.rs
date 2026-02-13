use std::{error::Error, path::Path};

mod diagram;
mod disallowed;
mod files;
mod fix;
mod format;
mod root;
mod rules;
pub mod ts_reader;
mod violations;
mod visit;

pub use diagram::update_diagrams_recursively;
pub use diagram::update_readme_with_diagram;
pub use fix::{fix_violation, remove_reference_to_nonexistent_directory};
pub use format::format_rules_file;
pub use format::format_rules_files_recursively;
pub use root::find_package_json_directory;
pub use rules::RULES_FILE_NAME;
pub use violations::{pretty_print_violations, DisallowedImportViolation, Violation};

pub fn list_violations(
    root: &Path,
    target: &Path,
    abort_on_violation: bool,
) -> Result<Vec<violations::Violation>, Box<dyn Error>> {
    let disallowed_imports = disallowed::get_initial_disallowed_imports(root, target);
    let mut violations = Vec::new();
    visit::visit_path(
        &mut violations,
        root,
        &disallowed_imports,
        target,
        abort_on_violation,
    )?;
    Ok(violations)
}
