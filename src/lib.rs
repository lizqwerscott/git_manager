use std::path::{Path, PathBuf};
use std::process;

pub type BDError = Box<dyn std::error::Error>;
pub type BDEResult<T> = Result<T, BDError>;

#[derive(Debug, Clone)]
pub struct GitError {
    err: String,
}

impl GitError {
    pub fn new(err: &str) -> GitError {
        GitError {
            err: err.to_string(),
        }
    }
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.err)
    }
}

impl std::error::Error for GitError {
    fn description(&self) -> &str {
        &self.err
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        // 泛型错误。没有记录其内部原因。
        None
    }
}

pub fn ba_error(error: &str) -> Box<dyn std::error::Error> {
    Box::new(GitError::new(error))
}

#[derive(Debug)]
enum GitStatus {
    Another,
}

#[derive(Debug)]
pub struct GitRepo {
    name: String,
    path: PathBuf,
    status: GitStatus,
}

pub fn run_command(command: &str) -> BDEResult<String> {
    match process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .output()
    {
        Ok(output) => Ok(String::from_utf8(output.stdout).unwrap()),
        Err(error) => Err(ba_error(
            format!("执行命令失败: {}", error.to_string()).as_mut_str(),
        )),
    }
}

pub fn search_all_git_repo(path: &Path) -> BDEResult<Vec<GitRepo>> {
    let ignore_dir = vec![".cache", ".local", ".cargo"];
    let search_git_str = "^\\..*git$";

    let ignore_dir_str: Vec<String> = ignore_dir
        .into_iter()
        .map(|item| format!("-E {}", item))
        .collect();

    let command = format!(
        "fd -t d -H {} '{}' {}",
        ignore_dir_str.join(" "),
        search_git_str,
        path.display()
    );

    println!("command: {}", command);
    let find_res = run_command(&command)?;
    let git_repos: Vec<GitRepo> = find_res
        .split('\n')
        .map(|path| {
            let path = Path::new(path);
            path.parent()
        })
        .filter_map(|path| {
            if let Some(path) = path {
                let file_name = path.file_name().unwrap().to_str().unwrap();

                Some(GitRepo {
                    name: String::from(file_name),
                    path: PathBuf::from(path),
                    status: GitStatus::Another,
                })
            } else {
                None
            }
        })
        .collect();

    Ok(git_repos)
}
