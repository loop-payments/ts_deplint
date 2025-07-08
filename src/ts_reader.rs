use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::LazyLock;

const IGNORE_COMMENT: &str = "// ts_deplint ignore";

pub fn read_ts_imports(ts_path: &Path) -> io::Result<Vec<String>> {
    let ts_file = File::open(ts_path)?;
    let reader = io::BufReader::new(ts_file);

    let mut ts_imports = Vec::new();

    let mut curr_line: String = "".to_string();
    let mut prev_line: String;
    for line in reader.lines() {
        prev_line = curr_line;
        curr_line = line?;

        if prev_line.contains(IGNORE_COMMENT) {
            continue;
        }

        if let Some(ts_import) = extract_import(&curr_line) {
            ts_imports.push(ts_import);
        }
    }

    Ok(ts_imports)
}

static IMPORT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?:from|import)+\s+["']([^"']+)["'];"#).unwrap());

fn extract_import(line: &str) -> Option<String> {
    let captures = IMPORT_REGEX.captures(line)?;
    let group_1 = captures.get(1)?;
    Some(group_1.as_str().to_string())
}

#[test]
fn test_extract_import_paths() {
    let cases = [
        ("import x from 'foo';", Some("foo")),
        ("import { y } from './bar';", Some("./bar")),
        ("import * as z from 'baz';", Some("baz")),
        ("import 'side-effect';", Some("side-effect")),
        (
            "import {\n    a,\n    b,\n    c,\n} from 'multi-line/import';",
            Some("multi-line/import"),
        ),
        (
            "import a from '@package/with-symbol';",
            Some("@package/with-symbol"),
        ),
        (
            "import a from \"@double-quote/import\";",
            Some("@double-quote/import"),
        ),
        ("function test() {", None),
    ];
    for (input, expected) in cases {
        assert_eq!(
            extract_import(input),
            expected.map(String::from),
            "Failed on input: {input}"
        );
    }
}
