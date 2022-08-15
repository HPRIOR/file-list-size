#![allow(dead_code, unused_variables)]

use std::{
    fs, os,
    path::{Path, PathBuf},
    process::{exit, Command},
};

use itertools::Itertools;

#[derive(Debug)]
struct File {
    full_path: String,
    path_to_file: Vec<String>,
    byte_size: f64,
}

#[derive(Debug)]
struct FileTree {
    files: Vec<File>,
    dir: Option<String>,
    tree_nodes: Vec<Box<FileTree>>,
}

impl FileTree {
    fn new(dir: Option<String>) -> Self {
        Self {
            files: vec![],
            dir,
            tree_nodes: vec![],
        }
    }

    fn populate(&mut self, file_paths: &Vec<File>, depth: usize) {
        file_paths
            .into_iter()
            .unique_by(|f| f.path_to_file.get(depth))
            .for_each(
                |File {
                     full_path,
                     path_to_file,
                     byte_size,
                 }| {
                    path_to_file.get(depth).map(|fd| match fd {
                        path_node if node_is_file(path_to_file, depth) => {
                            match self.dir.as_ref() {
                                Some(dir) => {
                                    if full_path.contains(dir) {
                                        self.files.push(File {
                                            full_path: full_path.clone(),
                                            path_to_file: path_to_file.clone(),
                                            byte_size: *byte_size,
                                        })
                                    }
                                }
                                None => self.files.push(File {
                                    full_path: full_path.clone(),
                                    path_to_file: path_to_file.clone(),
                                    byte_size: *byte_size,
                                }),
                            };
                        }
                        path_node => self
                            .tree_nodes
                            .push(Box::new(FileTree::new(Some(path_node.clone())))),
                    });
                },
            );


        for node in &mut self.tree_nodes {
            node.populate(file_paths, depth + 1);
        }
    }
}

fn node_is_file(path_to_file: &Vec<String>, depth: usize) -> bool {
    let truncated_path = &path_to_file[..=depth];
    let path: PathBuf = truncated_path.iter().collect();
    let metadata = std::fs::metadata(path).unwrap();
    metadata.is_file()
}

fn main() {
    let cmd_out = Command::new("sh")
        .arg("-c")
        .arg("git ls-files --others --exclude-standard")
        .output();
    match cmd_out {
        Ok(cmd) => {
            let std_out = String::from_utf8(cmd.stdout).unwrap();
            let file_list = get_file_list_from(&std_out);
            if file_list.len() == 0 {
                exit_with("No untracked files found")
            }
            let files = get_file_sizes_from(&file_list);
            let mut tree = FileTree::new(None);
            tree.populate(&files, 0);
            dbg!(tree);
        }
        Err(err) => {
            exit_with(format!("Git command failed: {}", err).as_str());
        }
    }
}

fn get_file_list_from(std_out: &String) -> Vec<String> {
    std_out
        .split("\n")
        .filter(|x| *x != "")
        .map(|x| x.to_owned())
        .collect()
}

fn get_file_sizes_from(file_list: &Vec<String>) -> Vec<File> {
    file_list
        .iter()
        .map(|x| {
            let size = fs::metadata(x).unwrap().len() as f64;
            File {
                byte_size: size,
                path_to_file: x
                    .split(std::path::MAIN_SEPARATOR)
                    .map(|s| s.to_owned())
                    .collect(),
                full_path: x.clone(),
            }
        })
        .collect()
}

fn size_str(size: f64) -> String {
    match size {
        x if x >= 0.0 && x <= 999.9 => {
            format!("{}b", truncate_decimal(&x.to_string()))
        }
        x if x >= 1000.0 && x <= 999_999.9 => {
            format!("{}kb", truncate_decimal(&(x / 1000.0).to_string()))
        }
        x if x >= 1_000_000.0 && x <= 999_999_999.9 => {
            format!("{}mb", truncate_decimal(&(x / 1_000_000.0).to_string()))
        }
        x => {
            format!("{}gb", truncate_decimal(&(x / 1_000_000_000.0).to_string()))
        }
    }
}

fn truncate_decimal(s: &String) -> String {
    let decimal_index = s
        .chars()
        .enumerate()
        .filter(|(_, ch)| *ch == '.')
        .nth(0)
        .map(|(i, _)| i);
    if let Some(index) = decimal_index {
        return s.clone()[..index + 2].to_owned();
    } else {
        s.clone()
    }
}

fn exit_with(msg: &str) {
    eprint!("{}", msg);
    exit(1);
}

#[cfg(test)]
mod tests {
    use crate::size_str;

    #[test]
    fn representation_of_bytes() {
        assert_eq!("100b", size_str(100.0))
    }

    #[test]
    fn representation_of_bytes_on_margin() {
        assert_eq!("999.9b", size_str(999.9))
    }

    #[test]
    fn representation_of_kb() {
        assert_eq!("1kb", size_str(1000.0))
    }
    #[test]
    fn representation_of_kb_on_margin() {
        assert_eq!("999.9kb", size_str(999_999.9))
    }
    #[test]
    fn representation_of_mb() {
        assert_eq!("1mb", size_str(1_000_000.0))
    }
    #[test]
    fn representation_of_mb_on_margin() {
        assert_eq!("999.9mb", size_str(999_999_999.9))
    }
}
