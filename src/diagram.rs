extern crate serde_yaml;

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    hash::Hash,
    io::{self, Write},
    path::Path,
};

use crate::rules::read_rules_file;
use crate::RULES_FILE_NAME;

type AllowsMap<T> = BTreeMap<T, BTreeSet<T>>;

/// Recursively find directories containing a rules file and update the diagram.
pub fn update_diagrams_recursively(
    dir: &Path,
    show_circular_dependencies: bool,
) -> Result<(), Box<dyn Error>> {
    if dir.join(RULES_FILE_NAME).exists() {
        let readme_path = dir.join("README.md");
        update_readme_with_diagram(
            &dir.join(RULES_FILE_NAME),
            &readme_path,
            show_circular_dependencies,
        )?;
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            update_diagrams_recursively(&path, show_circular_dependencies)?;
        }
    }
    Ok(())
}

fn get_allows(yaml_path: &Path) -> Result<AllowsMap<String>, Box<dyn Error>> {
    let yaml_rules = read_rules_file(yaml_path)?;
    let converted_rules = yaml_rules
        .allow
        .into_iter()
        .map(|(source, targets)| (source, BTreeSet::from_iter(targets.into_iter())))
        .collect::<AllowsMap<_>>();
    Ok(converted_rules)
}

fn get_transitive_allows<T>(direct_allows: &AllowsMap<T>) -> AllowsMap<&T>
where
    T: Eq + Hash + Ord,
{
    let mut transitive_allows = AllowsMap::new();
    for (source, targets) in direct_allows.iter() {
        if !transitive_allows.contains_key(source) {
            transitive_allows.insert(source, BTreeSet::new());
        }
        let mut to_visit = BTreeSet::from_iter(targets.iter());
        let mut seen = BTreeSet::new();
        while let Some(current) = to_visit.pop_first() {
            if seen.contains(current) {
                continue;
            }
            seen.insert(current);
            if let Some(reachable_from_source) = transitive_allows.get_mut(source) {
                reachable_from_source.insert(current);
                if let Some(allows) = direct_allows.get(current) {
                    to_visit.extend(allows);
                }
            }
        }
    }
    transitive_allows
}

#[test]
fn test_get_transitive_allows() {
    assert_eq!(
        get_transitive_allows(&AllowsMap::from([
            ("a", BTreeSet::from(["b"])),
            ("b", BTreeSet::from(["c"]))
        ])),
        AllowsMap::from([
            (&"a", BTreeSet::from([&"b", &"c"])),
            (&"b", BTreeSet::from([&"c"])),
        ])
    );
    assert_eq!(
        get_transitive_allows(&AllowsMap::from([
            ("a", BTreeSet::from(["b"])),
            ("b", BTreeSet::from(["c"])),
            ("c", BTreeSet::from(["a"]))
        ])),
        AllowsMap::from([
            (&"a", BTreeSet::from([&"a", &"b", &"c"])),
            (&"b", BTreeSet::from([&"a", &"b", &"c"])),
            (&"c", BTreeSet::from([&"a", &"b", &"c"])),
        ])
    );
}

fn get_other_readme_lines(readme_path: &Path) -> io::Result<(Vec<String>, Vec<String>)> {
    match fs::read_to_string(readme_path) {
        Ok(readme) => {
            let readme_lines: Vec<String> = readme.lines().map(|s| s.to_string()).collect();
            let dep_sigil_index = readme_lines
                .iter()
                .position(|line| line.starts_with("%%dep"));
            match dep_sigil_index {
                Some(dep_sigil_idx) => {
                    let block_start_idx = dep_sigil_idx.saturating_sub(1);
                    let block_end_idx = readme_lines[block_start_idx + 1..]
                        .iter()
                        .position(|line| line.starts_with("```"))
                        .map(|idx| idx + block_start_idx + 1)
                        .unwrap_or_else(|| readme_lines.len());

                    let before_dep_diagram_block = readme_lines[0..block_start_idx].to_vec();
                    let after_dep_diagram_block = readme_lines[block_end_idx + 1..].to_vec();

                    Ok((before_dep_diagram_block, after_dep_diagram_block))
                }
                None => Ok((readme_lines.clone(), vec![])),
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok((vec![], vec![])),
        Err(e) => Err(e),
    }
}

pub fn update_readme_with_diagram(
    yaml_path: &Path,
    readme_path: &Path,
    show_circular_dependencies: bool,
) -> Result<(), Box<dyn Error>> {
    let allows = get_allows(yaml_path)?;
    if allows.is_empty() {
        return Ok(());
    }
    let transitive_allows: AllowsMap<_> = get_transitive_allows(&allows);
    let (before_dep_diagram_block, after_dep_diagram_block) = get_other_readme_lines(readme_path)?;

    let mut circular_edge_indices = vec![];
    let mut mermaid_edges = vec![];
    for (source, targets) in &allows {
        for target in targets {
            if target == "-" {
                continue;
            }
            let is_circular_dependency = transitive_allows
                .get(target)
                .map(|deps| deps.contains(source))
                .unwrap_or(false);
            if is_circular_dependency {
                circular_edge_indices.push(format!("{}", mermaid_edges.len()));
            }
            mermaid_edges.push(format!("  {} --> {}", source, target));
        }
    }

    let mut output_lines = Vec::new();
    output_lines.extend(before_dep_diagram_block);
    output_lines.push("```mermaid".to_string());
    output_lines.push("%%dep".to_string());
    output_lines.push("graph TD".to_string());
    output_lines.push("  subgraph \" \"".to_string());
    output_lines.extend(mermaid_edges);
    output_lines.push("  end".to_string());
    if show_circular_dependencies && !circular_edge_indices.is_empty() {
        output_lines.push(format!(
            "linkStyle {} stroke:red;",
            circular_edge_indices.join(",")
        ));
    }
    output_lines.push("```".to_string());
    output_lines.extend(after_dep_diagram_block);

    // Add a newline to the end of the file if it doesn't already have one.
    if !output_lines.is_empty() {
        let last_line = output_lines.last().unwrap();
        if !last_line.is_empty() {
            output_lines.push("".to_string());
        }
    }

    let output_content = output_lines.join("\n");

    let mut file = fs::File::create(readme_path)?;
    file.write_all(output_content.as_bytes())?;

    Ok(())
}
