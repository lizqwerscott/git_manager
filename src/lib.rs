use std::path::{Path, PathBuf};
use std::process::{self, Stdio};
use std::time::Duration;
use std::fmt;

use tokio::process::Command;
use tokio::signal::ctrl_c;
use tokio::time::timeout;

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

#[derive(Debug, Clone, Copy)]
enum GitStatus {
    Clean,
    NeedPull,
    NeedPush,
    NeedCommit,
    Another,
}

impl fmt::Display for GitStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GitStatus::Clean => write!(f, "干净"),
            GitStatus::NeedPull => write!(f, "需要拉取"),
            GitStatus::NeedPush => write!(f, "需要推送"),
            GitStatus::NeedCommit => write!(f, "需要Commit"),
            GitStatus::Another => write!(f, "其它"),
        }
    }
}

#[derive(Debug)]
pub struct GitRepo {
    pub name: String,
    path: PathBuf,
    status: GitStatus,
}

impl GitRepo {
    pub async fn refresh_status(&mut self) -> BDEResult<()> {
        let status_res = run_command(format!("cd {} && git status", self.path.display()).as_str())?;
        let working_tree_clean = status_res.contains("working tree clean");

        if working_tree_clean {
            self.status = GitStatus::Clean;

            let have_remote =
                !run_command(format!("cd {} && git remote show", self.path.display()).as_str())?
                    .is_empty();

            if have_remote {
                run_command_timeout(
                    format!("cd {} && git fetch", self.path.display()).as_str(),
                    5,
                )
                .await?;
                let status_after_fetch_res =
                    run_command(format!("cd {} && git status", self.path.display()).as_str())?;
                let need_pull = status_after_fetch_res.contains("git pull");
                if need_pull {
                    self.status = GitStatus::NeedPull;
                }

                let need_push = status_after_fetch_res.contains("git push");
                if need_push {
                    self.status = GitStatus::NeedPush;
                }
            }
        } else {
            self.status = GitStatus::NeedCommit;
        }

        Ok(())
    }

    pub fn get_last_commit_time(&self) -> u64 {
        // println!("get commit: {}", self.path.display());

        let res = run_command(
            format!(
                "cd {} && git show --pretty=format:'%ct' | head -1",
                self.path.display()
            )
            .as_str(),
        )
        .unwrap();
        // println!("{}: commit time: {}", self.path.display(), res);
        if res.trim().is_empty() {
            0
        } else {
            res.trim().parse().unwrap()
        }
    }
}

impl fmt::Display for GitRepo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:20}: {:10}",
            self.name,
            self.status.to_string()
        )
    }
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

pub async fn run_command_timeout(command: &str, timeout_second: u64) -> BDEResult<String> {
    let timeout_duration = Duration::from_secs(timeout_second);

    let mut child = Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())  // 捕获标准输出
        .stderr(Stdio::null())   // 将标准错误重定向到空
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    // Create a future that resolves when Ctrl+C is pressed
    let ctrl_c_future = ctrl_c();

    tokio::select! {
        // Wait for the command to complete
        _ = child.wait() => {
            let output = child.wait_with_output().await?;
            if output.status.success() {
                Ok(String::from_utf8(output.stdout).unwrap())
            } else {
                Err(format!("Command failed with exit code: {}", output.status).into())
            }
        }

        // Wait for Ctrl+C or timeout
        _ = timeout(timeout_duration, ctrl_c_future) => {
            child.kill().await?;
            Err(ba_error("Command timed out"))
        }
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
    let mut git_repos: Vec<GitRepo> = find_res
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

    git_repos.sort_by(|a, b| b.get_last_commit_time().cmp(&a.get_last_commit_time()));

    Ok(git_repos)
}
