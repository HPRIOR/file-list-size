use std::{
    collections::HashSet,
    error::Error,
    fmt::Debug,
    fs,
    hash::Hash,
    os::unix::prelude::MetadataExt,
    process::{exit, Command},
};

use itertools::Itertools;

#[derive(Debug, Clone)]
struct File {
    path: String,
    byte_size: u64,
}

#[derive(Debug)]
struct FileTree {
    dir: String,
    files: Vec<File>,
    nodes: Vec<Box<FileTree>>,
    size: u64,
}

struct FileTreeInfo {
    dir: String,
    files: Vec<File>,
    size: u64,
}

impl From<&FileTree> for FileTreeInfo {
    fn from(file_tree: &FileTree) -> Self {
        Self {
            dir: file_tree.dir.clone(),
            files: file_tree.files.iter().map(|f| f.clone()).collect(),
            size: file_tree.size.clone(),
        }
    }
}

impl FileTree {
    fn new(
        dir_matrix: &Vec<Vec<String>>,
        target_files: &HashSet<String>,
        h_index: usize,
        w_index: usize,
    ) -> Option<Self> {
        let tree_height = dir_matrix.len();
        let children_index = h_index + 1;
        if children_index > tree_height {
            return None;
        }
        // use option instead?
        let no_children: Vec<String> = Vec::new();
        let children: &Vec<String> = if children_index == tree_height {
            &no_children
        } else {
            &dir_matrix[h_index + 1]
        };

        let dir = &dir_matrix[h_index][w_index];
        let child_nodes: Vec<Box<FileTree>> = children
            .into_iter()
            .enumerate()
            .filter(|(_, ch)| ch.contains(dir))
            .filter_map(|(i, _)| FileTree::new(dir_matrix, target_files, h_index + 1, i))
            .map(|ft| Box::new(ft))
            .collect();

        let files_in_cur_dir = get_files_at_dir(target_files, dir);
        let size_of_cur_dir = &files_in_cur_dir
            .iter()
            .fold(0, |acc, file| acc + file.byte_size);
        let size_of_children = &child_nodes.iter().fold(0, |acc, tree| acc + tree.size);

        Some(Self {
            dir: dir.clone(),
            files: get_files_at_dir(target_files, &dir),
            nodes: child_nodes,
            size: size_of_cur_dir + size_of_children,
        })
    }

    fn flatten(&self, tree_info: &mut Vec<FileTreeInfo>) {
        tree_info.push(self.into());
        self.nodes.iter().for_each(|node| node.flatten(tree_info));
    }

    fn print(&self) -> () {
        let mut file_tree_infos: Vec<FileTreeInfo> = vec![];
        self.flatten(&mut file_tree_infos);
        let sorted_info = file_tree_infos.iter().sorted_by(|a, b| a.size.cmp(&b.size));
        sorted_info.for_each(|inf| {
            let sorted_files = inf
                .files
                .iter()
                .sorted_by(|a, b| a.byte_size.cmp(&b.byte_size));
            println!("{}: {}", inf.dir, size_str(inf.size as f64));
            sorted_files
                .for_each(|f| println!("    - {}: {}", f.path, size_str(f.byte_size as f64)))
        })
    }
}

fn get_dir_hierarchy_matrix(files: &Vec<String>) -> Vec<Vec<String>> {
    let paths: Vec<Vec<String>> = files
        .into_iter()
        .map(|s| {
            s.split(std::path::MAIN_SEPARATOR)
                .map(|s| s.to_owned())
                .scan(String::from(""), |acc, next| {
                    acc.push(std::path::MAIN_SEPARATOR);
                    acc.push_str(next.as_str());
                    Some(acc.to_owned())
                })
                .collect()
        })
        .collect();

    dir_tree_hierarchy(&paths)
}

fn get_files_at_dir(untracked_files: &HashSet<String>, dir_path: &String) -> Vec<File> {
    fs::read_dir(format!(".{}", dir_path.as_str()))
        .unwrap()
        .map(|de| {
            let path = de.as_ref().unwrap().path();
            let meta = de.unwrap().metadata().unwrap();
            (path, meta)
        })
        .filter(|(_, meta)| meta.is_file())
        .map(|(pb, meta)| (pb.to_str().unwrap().to_owned(), meta))
        .filter(|(fp, _)| untracked_files.contains(&fp[2..]))
        .map(|(fp, meta)| File {
            path: fp,
            byte_size: meta.size(),
        })
        .collect()
}

fn dir_tree_hierarchy<T: Clone + Eq + Hash>(input: &Vec<Vec<T>>) -> Vec<Vec<T>> {
    let mut output: Vec<HashSet<T>> = vec![];
    for vec in input.iter() {
        let vec_length = vec.len();
        for i in 0..vec_length {
            // ignore files
            if i == vec_length - 1 {
                break;
            }
            let input_item = vec[i].clone();
            if let Some(set) = output.get_mut(i) {
                set.insert(input_item);
            } else {
                let mut new_vec = HashSet::new();
                new_vec.insert(input_item);
                output.push(new_vec);
            }
        }
    }

    output
        .into_iter()
        .map(|x| x.into_iter().collect())
        .collect()
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
fn execute() -> Result<(), Box<dyn Error>> {
    let cmd = Command::new("sh")
        .arg("-c")
        .arg("git ls-files --others --exclude-standard")
        .output()?;

    let std_out = String::from_utf8(cmd.stdout).unwrap();
    let file_list = get_file_list_from(&std_out);
    let file_hash_set: HashSet<String> = file_list.iter().map(|f| f.clone()).collect();

    let dir_matrix = get_dir_hierarchy_matrix(&file_list);
    let file_tree = FileTree::new(&dir_matrix, &file_hash_set, 0, 0);
    if let Some(file_tree) = file_tree {
        file_tree.print()
    }

    Ok(())
}

fn main() {
    let result = execute();
    match result {
        Ok(()) => exit(0),
        Err(err) => exit_with(format!("Git command failed: {}", err).as_str()),
    };
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
