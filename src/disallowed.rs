use std::path::Path;

use crate::rules::{self, Rules};

pub fn get_initial_disallowed_imports(root: &Path, target: &Path) -> Vec<String> {
    get_initial_disallowed_imports_impl(root, target, vec![], root)
}

pub fn get_child_disallowed_imports(
    root: &Path,
    current: &Path,
    disallowed_imports: &[String],
    rules: &Option<Rules>,
    directory: &str,
) -> Vec<String> {
    let mut dir_disallowed_imports = disallowed_imports.to_owned();
    if let Some(rules) = rules {
        if let Some(disallowed_siblings) = rules.get_disallowed_siblings(directory) {
            let new_disallowed_imports = disallowed_siblings
                .iter()
                .map(|s| current.join(s))
                .filter_map(|p| p.strip_prefix(root).ok().map(|p| p.to_path_buf()))
                .map(|p| {
                    let mut r = p.to_str().expect("").to_string();
                    // Include trailing slash. Say:
                    // src/foo/ is disallowed
                    // src/foo-bar/ is allowed
                    // Without the trailing slash, we'd incorrectly
                    // disallow foo-bar since it would match src/foo.
                    r.push('/');
                    r
                })
                .collect::<Vec<_>>();
            dir_disallowed_imports.extend(new_disallowed_imports);
        }
    }
    dir_disallowed_imports
}

fn get_initial_disallowed_imports_impl(
    root: &Path,
    target: &Path,
    disallowed_imports: Vec<String>,
    current: &Path,
) -> Vec<String> {
    if target.eq(current) {
        return disallowed_imports;
    }
    let remainder = target
        .strip_prefix(current)
        .unwrap_or_else(|_| panic!("Failed to strip prefix {:?} from {:?}", current, target));
    let next_dir_name = remainder
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str().map(String::from))
        .expect("Failed to read next directory name.");
    let (rules, _) = &rules::get_dir_rules_if_exists(root, current);
    let child_disallowed_imports =
        get_child_disallowed_imports(root, current, &disallowed_imports, rules, &next_dir_name);
    get_initial_disallowed_imports_impl(
        root,
        target,
        child_disallowed_imports,
        &current.join(next_dir_name),
    )
}
