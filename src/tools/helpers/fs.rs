use std::path::Path;

pub fn is_path_in_workspace<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    if path.as_ref().is_absolute() {
        return false;
    }

    for component in path.as_ref().components() {
        if component == std::path::Component::ParentDir {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_path_in_workspace_works() {
        // GIVEN
        let cases = vec![
            ("relative simple path", "file.txt", true),
            ("relative nested path", "src/main.rs", true),
            ("current directory reference", "./file.txt", true),
            ("empty path", "", true),
            ("path with dots in name", "file.tar.gz", true),
            ("absolute path", "/file.txt", false),
            ("path with parent directory", "src/../main.rs", false),
            ("parent directory only", "..", false),
            ("pure parent traversal", "../../..", false),
        ];

        // WHEN
        // THEN
        for (name, path, expected) in cases {
            let result = is_path_in_workspace(path);
            assert_eq!(
                result, expected,
                "test '{}' failed: expected {}, got {} for path '{}'",
                name, expected, result, path
            );
        }
    }
}
