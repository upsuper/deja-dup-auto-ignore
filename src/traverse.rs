use crate::stack_vec::StackVec;
use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use log::{debug, warn};
use std::fs;
use std::path::Path;

pub type Callback<'a> = dyn FnMut(&Path) + 'a;

pub fn find_directory_to_ignore(
    root: &Path,
    exclude_paths: &[&Path],
    cb: &mut Callback<'_>,
) -> Result<()> {
    let mut gitignore_stack = Vec::new();
    traverse(root, exclude_paths, &mut gitignore_stack, cb)?;
    Ok(())
}

fn traverse(
    dir: &Path,
    exclude_paths: &[&Path],
    gitignore_stack: &mut Vec<Gitignore>,
    cb: &mut Callback<'_>,
) -> Result<()> {
    let mut next_exclude_paths = Vec::new();
    for exclude_path in exclude_paths {
        if dir == *exclude_path || dir.starts_with(exclude_path) {
            debug!("Skipping excluded path: {}", dir.display());
            return Ok(());
        } else if exclude_path.starts_with(dir) {
            next_exclude_paths.push(*exclude_path);
        }
    }

    if dir.join(".deja-dup-ignore").exists() || dir.join("CACHEDIR.TAG").exists() {
        debug!(
            "Skipping directory with existing ignore file: {}",
            dir.display()
        );
        return Ok(());
    }

    let mut is_ignored = false;
    for gitignore in gitignore_stack.iter() {
        let matched = gitignore.matched(dir, true);
        if matched.is_ignore() {
            is_ignored = true;
            break;
        }
    }
    if is_ignored {
        cb(dir);
        return Ok(());
    }

    let mut gitignore_stack = StackVec::new(gitignore_stack);
    if let Some(gitignore) = maybe_build_gitignore(dir)? {
        gitignore_stack.push(gitignore);
    }

    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.collect(),
        Err(e) => {
            warn!("Failed to read directory {}: {}", dir.display(), e);
            return Ok(());
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read entry: {}", e);
                continue;
            }
        };

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(e) => {
                warn!("Failed to get file type: {}", e);
                continue;
            }
        };
        if !file_type.is_dir() {
            continue;
        }
        // gitignore doesn't apply to .git files
        if entry.file_name() == ".git" {
            continue;
        }

        let path = entry.path();
        traverse(&path, &next_exclude_paths, gitignore_stack.inner(), cb)?;
    }

    Ok(())
}

fn maybe_build_gitignore(dir: &Path) -> Result<Option<Gitignore>> {
    let mut builder = GitignoreBuilder::new(dir);
    let mut has_new_gitignore = false;

    let gitignore_path = dir.join(".gitignore");
    if gitignore_path.exists() {
        builder.add(&gitignore_path);
        has_new_gitignore = true;
    }

    has_new_gitignore
        .then(|| builder.build().context("Failed to build gitignore"))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct TestCase {
        name: &'static str,
        structure: &'static [(&'static str, FileType)],
        exclude_paths: &'static [&'static str],
        expected: &'static [&'static str],
    }

    #[derive(Debug, Clone, Copy)]
    enum FileType {
        Dir,
        File(&'static str),
    }

    impl TestCase {
        fn run(&self) {
            let temp_dir = tempfile::tempdir().unwrap();
            let root = temp_dir.path();

            // Create directory structure
            for (path, file_type) in self.structure {
                let full_path = root.join(path);
                match file_type {
                    FileType::Dir => {
                        fs::create_dir_all(&full_path).unwrap();
                    }
                    FileType::File(content) => {
                        if let Some(parent) = full_path.parent() {
                            fs::create_dir_all(parent).unwrap();
                        }
                        fs::write(&full_path, content).unwrap();
                    }
                }
            }

            // Collect results
            let mut results = Vec::new();
            let exclude_paths: Vec<_> = self.exclude_paths.iter().map(|p| root.join(p)).collect();
            let exclude_refs: Vec<_> = exclude_paths.iter().map(|p| p.as_path()).collect();

            find_directory_to_ignore(root, &exclude_refs, &mut |path| {
                let rel_path = path.strip_prefix(root).unwrap();
                results.push(rel_path.to_string_lossy().to_string());
            })
            .unwrap();

            // Verify results
            let expected_set: HashSet<_> = self.expected.iter().copied().collect();
            let results_set: HashSet<_> = results.iter().map(|s| s.as_str()).collect();

            assert_eq!(
                expected_set, results_set,
                "Test '{}' failed.\nExpected: {:?}\nGot: {:?}",
                self.name, self.expected, results
            );
        }
    }

    #[test]
    fn test_basic_gitignore() {
        TestCase {
            name: "basic gitignore",
            structure: &[
                ("a/.gitignore", FileType::File("ignored/")),
                ("a/ignored", FileType::Dir),
                ("a/not_ignored", FileType::Dir),
            ],
            exclude_paths: &[],
            expected: &["a/ignored"],
        }
        .run();
    }

    #[test]
    fn test_directory_only() {
        TestCase {
            name: "directory only",
            structure: &[
                ("a", FileType::Dir),
                ("a/.gitignore", FileType::File("ignored")),
                ("a/ignored", FileType::File("")),
            ],
            exclude_paths: &[],
            expected: &[],
        }
        .run();
    }

    #[test]
    fn test_nested_gitignore() {
        TestCase {
            name: "nested gitignore",
            structure: &[
                ("a", FileType::Dir),
                ("a/.gitignore", FileType::File("ignored/")),
                ("a/ignored", FileType::Dir),
                ("a/ignored/.gitignore", FileType::File("nested/")),
                ("a/ignored/nested", FileType::Dir),
                ("a/ignored/not_nested", FileType::Dir),
            ],
            exclude_paths: &[],
            expected: &["a/ignored"],
        }
        .run();
    }

    #[test]
    fn test_dotgit() {
        TestCase {
            name: ".git",
            structure: &[
                (".git", FileType::Dir),
                (".gitignore", FileType::File("ignored/")),
                (".git/ignored", FileType::Dir),
                (".git/not_ignored", FileType::Dir),
            ],
            exclude_paths: &[],
            expected: &[],
        }
        .run();
    }

    #[test]
    fn test_exclude_paths() {
        TestCase {
            name: "exclude paths",
            structure: &[
                (".gitignore", FileType::File("ignored/")),
                ("ignored", FileType::Dir),
                ("excluded", FileType::Dir),
                ("excluded/.gitignore", FileType::File("nested/")),
                ("excluded/nested", FileType::Dir),
            ],
            exclude_paths: &["excluded"],
            expected: &["ignored"],
        }
        .run();
    }

    #[test]
    fn test_deja_dup_ignore_skips_directory() {
        TestCase {
            name: "deja-dup-ignore skips directory",
            structure: &[
                ("a", FileType::Dir),
                ("a/.gitignore", FileType::File("ignored/")),
                ("a/ignored", FileType::Dir),
                ("a/ignored/.deja-dup-ignore", FileType::File("")),
                ("a/ignored/nested", FileType::Dir),
            ],
            exclude_paths: &[],
            expected: &[],
        }
        .run();
    }

    #[test]
    fn test_multiple_ignored_directories() {
        TestCase {
            name: "multiple ignored directories",
            structure: &[
                (
                    ".gitignore",
                    FileType::File("build/\ntarget/\nnode_modules/"),
                ),
                ("build", FileType::Dir),
                ("target", FileType::Dir),
                ("node_modules", FileType::Dir),
                ("src", FileType::Dir),
            ],
            exclude_paths: &[],
            expected: &["build", "target", "node_modules"],
        }
        .run();
    }

    #[test]
    fn test_gitignore_pattern_matching() {
        TestCase {
            name: "gitignore pattern matching",
            structure: &[
                (".gitignore", FileType::File("*.log\n*.tmp")),
                ("test.log", FileType::Dir),
                ("debug.tmp", FileType::Dir),
                ("test.txt", FileType::Dir),
            ],
            exclude_paths: &[],
            expected: &["test.log", "debug.tmp"],
        }
        .run();
    }

    #[test]
    fn test_no_gitignore() {
        TestCase {
            name: "no gitignore",
            structure: &[
                ("a", FileType::Dir),
                ("a/b", FileType::Dir),
                ("a/c", FileType::Dir),
            ],
            exclude_paths: &[],
            expected: &[],
        }
        .run();
    }

    #[test]
    fn test_gitignore_inheritance() {
        TestCase {
            name: "gitignore inheritance",
            structure: &[
                (".gitignore", FileType::File("*.log")),
                ("a", FileType::Dir),
                ("a/test.log", FileType::Dir),
                ("b", FileType::Dir),
                ("b/.gitignore", FileType::File("*.tmp")),
                ("b/test.log", FileType::Dir),
                ("b/test.tmp", FileType::Dir),
            ],
            exclude_paths: &[],
            expected: &["a/test.log", "b/test.log", "b/test.tmp"],
        }
        .run();
    }

    #[test]
    fn test_complex_scenario() {
        TestCase {
            name: "complex scenario",
            structure: &[
                (".gitignore", FileType::File("build/\n*.log")),
                ("build", FileType::Dir),
                ("src", FileType::Dir),
                ("src/.gitignore", FileType::File("target/")),
                ("src/target", FileType::Dir),
                ("src/main", FileType::Dir),
                ("test.log", FileType::Dir),
                ("excluded", FileType::Dir),
                ("excluded/.gitignore", FileType::File("ignored/")),
                ("excluded/ignored", FileType::Dir),
            ],
            exclude_paths: &["excluded"],
            expected: &["build", "src/target", "test.log"],
        }
        .run();
    }
}
