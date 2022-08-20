#![allow(dead_code, unused_variables)]

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::{self, DirEntry, Metadata},
    os::unix::prelude::MetadataExt,
    path::PathBuf,
    process::{exit, Command},
};

#[derive(Debug)]
struct File {
    path: String,
    byte_size: u64,
}

#[derive(Debug)]
struct FileTree {
    files: Vec<File>,
    dir: Option<String>,
    nodes: Vec<Box<FileTree>>,
}

impl FileTree {
    fn new(untracked_files: &HashSet<String>, path: &String) -> Option<Self> {
        let dir_info: Vec<(PathBuf, Metadata)> = fs::read_dir(path)
            .unwrap()
            .map(|de| {
                let path = de.as_ref().unwrap().path();
                let meta = de.unwrap().metadata().unwrap();
                (path, meta)
            })
            .collect();

        let dirs: Vec<String> = dir_info
            .iter()
            .filter(|(_, meta)| meta.is_dir())
            .map(|(p, _)| p.to_str().unwrap().to_owned())
            .collect();

        let files: Vec<File> = dir_info
            .iter()
            .filter(|(_, meta)| meta.is_file())
            .map(|(pb, meta)| (pb.to_str().unwrap().to_owned(), meta))
            .filter(|(fp, _)| untracked_files.contains(&fp[2..]))
            .map(|(fp, meta)| File {
                path: fp,
                byte_size: meta.size(),
            })
            .collect();

        println!("number of files in {} files: {:?}", path, &files);


        if files.len() > 0 || path == "./" {
            let nodes: Vec<Box<FileTree>> = dirs
                .into_iter()
                .filter_map(|dir| FileTree::new(untracked_files, &dir))
                .map(|ft| Box::new(ft))
                .collect();

            return Some(Self {
                files,
                dir: Some(path.clone()),
                nodes,
            });
        }

        None
    }
}

fn execute() -> Result<(), Box<dyn Error>> {
    let cmd = Command::new("sh")
        .arg("-c")
        .arg("git ls-files --others --exclude-standard")
        .output()?;

    let std_out = String::from_utf8(cmd.stdout).unwrap();
    let file_list = get_file_list_from(&std_out);
    let file_hash_set: HashSet<String> = file_list.iter().map(|f| f.clone()).collect();
    let file_tree = FileTree::new(&file_hash_set, &String::from("./"));
    println!("file tree:");
    println!("{:?}", file_tree);

    Ok(())
}

fn main() {
    let result = execute();
    match result {
        Ok(()) => exit(0),
        Err(err) => exit_with(format!("Git command failed: {}", err).as_str()),
    };
}

fn get_file_list_from(std_out: &String) -> Vec<String> {
    std_out
        .split("\n")
        .filter(|x| *x != "")
        .map(|x| x.to_owned())
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
