use ts_deplint::ts_reader;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_fixture_path() -> PathBuf {
        // Get the path to the fixtures directory
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("fixtures");
        path.push("sample.ts");
        path
    }

    #[test]
    fn test_read_ts_imports() {
        // Get the path to the fixture file
        let ts_path = get_fixture_path();

        // Call the function and check the result
        let result = ts_reader::read_ts_imports(&ts_path);
        assert!(result.is_ok());

        let ts_imports = result.unwrap();
        assert_eq!(ts_imports, vec!["foo", "baz/baz/baz", "./baz/bay"]);
    }
}
