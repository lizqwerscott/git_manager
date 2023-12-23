use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::task::JoinSet;

use crate::utils::{run_command, run_command_timeout_no, BDEResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum GitStatus {
    Clean,
    NeedPull,
    NeedPush,
    NeedCommit,
    Timeout,
}

impl fmt::Display for GitStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GitStatus::Clean => write!(f, "干净"),
            GitStatus::NeedPull => write!(f, "需要拉取"),
            GitStatus::NeedPush => write!(f, "需要推送"),
            GitStatus::NeedCommit => write!(f, "需要Commit"),
            GitStatus::Timeout => write!(f, "超时"),
            // GitStatus::Another => write!(f, "其它"),
        }
    }
}

// Implement the FromStr trait for the enum
impl FromStr for GitStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Clean" => Ok(GitStatus::Clean),
            "NeedPull" => Ok(GitStatus::NeedPull),
            "NeedPush" => Ok(GitStatus::NeedPush),
            "NeedCommit" => Ok(GitStatus::NeedCommit),
            "Timeout" => Ok(GitStatus::Timeout),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitRepo {
    pub name: String,
    pub path: PathBuf,
    pub status: GitStatus,
    pub last_commit_time: u64,
}

impl GitRepo {
    pub async fn build(path: &Path) -> BDEResult<Self> {
        let last_commit_time = GitRepo::get_last_commit_time(path)?;

        let status = match GitRepo::get_status(path).await {
            Ok(res) => res,
            Err(_) => GitStatus::Timeout,
        };

        let file_name = path.file_name().unwrap().to_str().unwrap();

        Ok(GitRepo {
            name: String::from(file_name),
            path: PathBuf::from(path),
            status,
            last_commit_time,
        })
    }

    pub async fn build_from_last(repo: GitRepo) -> BDEResult<Self> {
        let path = repo.path;

        let last_commit_time = GitRepo::get_last_commit_time(&path)?;
        let status = if repo.status != GitStatus::Timeout {
            match GitRepo::get_status(&path).await {
                Ok(res) => res,
                Err(_) => GitStatus::Timeout,
            }
        } else {
            GitStatus::Timeout
        };

        Ok(GitRepo {
            name: repo.name,
            path,
            status,
            last_commit_time,
        })
    }

    pub async fn get_status(path: &Path) -> BDEResult<GitStatus> {
        let status_res = run_command(format!("cd {} && git status", path.display()).as_str())?;
        let working_tree_clean = status_res.contains("working tree clean");

        Ok(if working_tree_clean {
            let mut new_status = GitStatus::Clean;

            let have_remote =
                !run_command(format!("cd {} && git remote show", path.display()).as_str())?
                    .is_empty();

            if have_remote {
                let mut now_need_pull = status_res.contains("git pull");
                let mut now_need_push = status_res.contains("git push");

                if !now_need_push && !now_need_pull {
                    run_command_timeout_no(
                        format!("cd {} && git fetch", path.display()).as_str(),
                        5,
                    )
                    .await?;
                    let status_after_fetch_res =
                        run_command(format!("cd {} && git status", path.display()).as_str())?;
                    now_need_pull = status_after_fetch_res.contains("git pull");
                    now_need_push = status_after_fetch_res.contains("git push");
                }

                if now_need_pull {
                    new_status = GitStatus::NeedPull;
                }

                if now_need_push {
                    new_status = GitStatus::NeedPush;
                }
            }

            new_status
        } else {
            GitStatus::NeedCommit
        })
    }

    pub fn get_last_commit_time(path: &Path) -> BDEResult<u64> {
        let res = run_command(
            format!(
                "cd {} && git show --pretty=format:'%ct' | head -1",
                path.display()
            )
            .as_str(),
        )?;
        // println!("{}: commit time: {}", self.path.display(), res);
        Ok(if res.trim().is_empty() {
            0
        } else {
            res.trim().parse()?
        })
    }
}

impl fmt::Display for GitRepo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:20}: {:10}", self.name, self.status.to_string())
    }
}

fn get_save_git_repo_path() -> BDEResult<PathBuf> {
    let repo_data_dir = PathBuf::from("/home/lizqwer/.cache/git_manager/");
    if !repo_data_dir.exists() {
        fs::create_dir_all(&repo_data_dir)?;
    }

    let repo_data_path = repo_data_dir.join("repo.json");

    Ok(repo_data_path)
}

fn search_all_git_path(search_path: &Path) -> BDEResult<Vec<PathBuf>> {
    let ignore_dir = vec![".cache", ".local", ".cargo", "clasp"];
    // 一旦 Fetch 在一些需要输入密码的情况下会导致仓库无法被删除
    let search_git_str = "^\\..*git$";

    let ignore_dir_str: Vec<String> = ignore_dir
        .into_iter()
        .map(|item| format!("-E {}", item))
        .collect();

    let command = format!(
        "fd -I -t d -H {} '{}' {}",
        ignore_dir_str.join(" "),
        search_git_str,
        search_path.display()
    );

    //println!("command: {}", command);
    let find_res = run_command(&command)?;
    let all_paths: Vec<PathBuf> = find_res
        .split('\n')
        .filter_map(|path| Path::new(path).parent().map(PathBuf::from))
        .collect();

    Ok(all_paths)
}

pub fn save_all_git_repo(repos: &[GitRepo]) -> BDEResult<()> {
    let repo_data_path = get_save_git_repo_path()?;

    let repo_data_json = serde_json::to_string_pretty(&repos)?;
    fs::write(repo_data_path, repo_data_json)?;
    Ok(())
}

pub fn load_all_repo() -> BDEResult<Option<Vec<GitRepo>>> {
    let repo_data_path = get_save_git_repo_path()?;

    if !repo_data_path.exists() {
        Ok(None)
    } else {
        let repo_data_json = fs::read_to_string(repo_data_path)?;
        let repos = serde_json::from_str(&repo_data_json)?;
        Ok(Some(repos))
    }
}

pub async fn generate_git_repo(all_paths: Vec<PathBuf>) -> BDEResult<(Vec<GitRepo>, u64)> {
    let mut set = JoinSet::new();
    for path in all_paths {
        // let path_str = path.display().to_string();

        set.spawn(async move {
            // let path = Path::new(&path_str);
            match GitRepo::build(&path).await {
                Ok(repo) => Some(repo),
                Err(err) => {
                    println!("build err({}): {}", path.display(), err);
                    None
                }
            }
        });
    }

    let mut git_repos: Vec<GitRepo> = Vec::new();
    let mut err_len = 0;
    while let Some(res) = set.join_next().await {
        match res {
            Ok(repo) => {
                if let Some(repo) = repo {
                    git_repos.push(repo);
                } else {
                    err_len += 1;
                }
            }
            Err(_) => {
                err_len += 1;
            }
        }
    }

    git_repos.sort_by_key(|item| item.last_commit_time);
    git_repos.reverse();

    Ok((git_repos, err_len))
}

pub async fn get_all_git_repo(search_path: &Path) -> BDEResult<(Vec<GitRepo>, u64)> {
    let mut all_git_paths = search_all_git_path(search_path)?;

    let repos = load_all_repo()?;
    let (res_repos, err_len) = if let Some(repos) = repos {
        // 刷新旧 Git repo 状态
        let mut set = JoinSet::new();
        for repo in repos {
            all_git_paths.retain(|item| item != &repo.path);

            set.spawn(async move {
                match GitRepo::build_from_last(repo).await {
                    Ok(repo) => Some(repo),
                    Err(_) => {
                        // println!("build err({}): {}", repo.path.display(), err);
                        None
                    }
                }
            });
        }

        // 将新增加的 Git repo 路径写入
        for path in all_git_paths {
            set.spawn(async move {
                match GitRepo::build(&path).await {
                    Ok(repo) => Some(repo),
                    Err(_) => {
                        // println!("build err({}): {}", repo.path.display(), err);
                        None
                    }
                }
            });
        }

        let mut git_repos: Vec<GitRepo> = Vec::new();
        let mut err_len = 0;
        while let Some(res) = set.join_next().await {
            match res {
                Ok(repo) => {
                    if let Some(repo) = repo {
                        git_repos.push(repo);
                    } else {
                        err_len += 1;
                    }
                }
                Err(_) => {
                    err_len += 1;
                }
            }
        }

        git_repos.sort_by_key(|item| item.last_commit_time);
        git_repos.reverse();

        (git_repos, err_len)
    } else {
        // 本地搜索
        let res = generate_git_repo(all_git_paths).await?;
        (res.0, res.1)
    };
    // 搜索完成之后保存
    save_all_git_repo(&res_repos)?;

    Ok((res_repos, err_len))
}

#[cfg(test)]
mod test {
    #[test]
    #[ignore]
    fn test() {}
}
