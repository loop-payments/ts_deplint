use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::io::Write;

use crate::violations::ReferenceToNonexistentDirectory;

pub const RULES_FILE_NAME: &str = ".deplint.rules.yml";

#[derive(Serialize, Deserialize)]
pub struct Rules {
    pub allow: BTreeMap<String, Vec<String>>,
}

impl Rules {
    // Constructor that takes allow as a parameter.
    pub fn new_with_allow(allow: BTreeMap<String, Vec<String>>) -> Self {
        Self { allow }
    }

    /// Returns a vector of sibling directory names that code in the
    /// passed-in directory is disallowed to import.
    pub fn get_disallowed_siblings(&self, dirname: &str) -> Option<Vec<&str>> {
        let unique_dirs = self.extract_unique_dirs();
        let allowed_dirs = self.get_allowed_siblings(dirname).unwrap_or(vec![]);
        let diff = find_difference(&unique_dirs, &allowed_dirs);
        let diff = diff
            .into_iter()
            .filter(|x| *x != dirname)
            .collect::<Vec<_>>();
        Some(diff)
    }

    fn extract_unique_dirs(&self) -> Vec<&str> {
        let mut unique_names = Vec::with_capacity(self.allow.len());
        for (key, names) in self.allow.iter() {
            unique_names.push(key.as_str());
            for name in names {
                unique_names.push(name.as_str());
            }
        }
        unique_names.sort();
        unique_names.dedup();
        unique_names
    }

    fn get_allowed_siblings(&self, dirname: &str) -> Option<Vec<&str>> {
        let siblings = self.allow.get(dirname)?;
        Some(siblings.iter().map(|s| s.as_str()).collect())
    }
}

fn find_difference<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    a.iter()
        .filter(|x| !b.contains(x))
        .copied()
        .collect::<Vec<&str>>()
}

pub fn get_dir_rules_if_exists(
    root: &Path,
    dir_path: &Path,
) -> (Option<Rules>, Vec<ReferenceToNonexistentDirectory>) {
    let rules_path = dir_path.join(RULES_FILE_NAME);
    match read_rules_file(&rules_path) {
        Ok(rules) => {
            let issues = lint_rules_file(root, dir_path, &rules_path, &rules);
            (Some(rules), issues)
        }
        Err(_e) => (None, vec![]),
    }
}

fn lint_rules_file(
    root: &Path,
    dir_path: &Path,
    rules_path: &Path,
    rules: &Rules,
) -> Vec<ReferenceToNonexistentDirectory> {
    let mut issues = vec![];
    let relative_rules_path = rules_path.strip_prefix(root).unwrap_or(rules_path);
    for (source, targets) in &rules.allow {
        let source_path = Path::new(dir_path).join(source);
        if !source_path.is_dir() {
            issues.push(ReferenceToNonexistentDirectory {
                directory_name: source.to_string(),
                file_path: relative_rules_path.to_str().unwrap().to_string(),
            })
        }
        for target in targets {
            if target == "-" {
                continue;
            }
            let target_path = Path::new(dir_path).join(target);
            if !target_path.is_dir() {
                issues.push(ReferenceToNonexistentDirectory {
                    directory_name: target.to_string(),
                    file_path: relative_rules_path.to_str().unwrap().to_string(),
                });
            }
        }
    }
    issues
}

pub fn read_rules_file(path: &Path) -> Result<Rules, Box<dyn Error>> {
    let file = File::open(path)?;
    let rules: Rules = serde_yaml::from_reader(file)?;
    Ok(rules)
}

pub fn write_formatted_rules_file(path: &Path, rules: Rules) -> Result<(), Box<dyn Error>> {
    let mut f = File::create(path)?;
    // Sort the keys within the allow map.
    let mut new_allow = BTreeMap::new();
    for (key, mut values) in rules.allow.into_iter() {
        values.sort();
        new_allow.insert(key, values);
    }
    let new_rules = Rules::new_with_allow(new_allow);
    let yaml_content = serde_yaml::to_string(&new_rules)?;
    // Replace " with '.
    let yaml_content = yaml_content.replace("\"", "'");
    f.write_all(yaml_content.as_bytes())?;
    Ok(())
}
