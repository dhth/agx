use std::path::{Component, Path};

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

pub fn path_to_dirname<P>(path: P) -> String
where
    P: AsRef<Path>,
{
    path.as_ref()
        .components()
        .filter_map(|c| match c {
            Component::Normal(os_str) => os_str.to_str(),
            _ => None,
        })
        .map(|s| s.replace(char::is_whitespace, "-"))
        .collect::<Vec<_>>()
        .join("-")
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

    #[test]
    fn path_to_dirname_works() {
        // GIVEN
        let cases = vec![
            (
                "absolute simple path",
                "/Users/user/file.txt",
                "Users-user-file.txt",
            ),
            (
                "absolute nested path",
                "/Users/user/projects/agx",
                "Users-user-projects-agx",
            ),
            (
                "absolute with spaces",
                "/Users/John Doe/projects/my app",
                "Users-John-Doe-projects-my-app",
            ),
            ("relative simple filename", "file.txt", "file.txt"),
            ("relative nested path", "src/main.rs", "src-main.rs"),
            (
                "relative with spaces",
                "my folder/my file.txt",
                "my-folder-my-file.txt",
            ),
            ("current directory prefix", "./file.txt", "file.txt"),
            ("single directory", "src", "src"),
            ("path with dots in filename", "file.tar.gz", "file.tar.gz"),
            (
                "multiple spaces",
                "/Users/user/my  app",
                "Users-user-my--app",
            ),
            ("empty path", "", ""),
        ];

        // WHEN
        // THEN
        for (name, path, expected) in cases {
            let result = path_to_dirname(path);
            assert_eq!(
                result, expected,
                "test '{}' failed: expected '{}', got '{}' for path '{}'",
                name, expected, result, path
            );
        }
    }
}
