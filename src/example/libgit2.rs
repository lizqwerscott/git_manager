use git2::{Cred, CredentialType, Error, Repository};
use std::collections::HashMap;

    pub fn get_remote_branch_remote_name(
        remote_branch: &git2::Branch<'_>,
    ) -> BDEResult<Option<String>> {
        let branch_name = remote_branch.name()?;
        Ok(branch_name.and_then(|branch_name| {
            let temp_split: Vec<&str> = branch_name.split('/').collect();
            let name = temp_split.first();
            name.map(|name| String::from(*name))
        }))
    }


    pub fn get_all_remote_and_branchs(
        repo: &Repository,
    ) -> BDEResult<HashMap<String, Vec<String>>> {
        let mut res: HashMap<String, Vec<String>> = HashMap::new();

        let mut remote_names: Vec<String> = Vec::new();
        let mut local_branch_names: Vec<String> = Vec::new();

        let remotes = repo.remotes()?;

        for remote_name in remotes.iter() {
            if let Some(remote_name) = remote_name {
                remote_names.push(String::from(remote_name));
            }
        }

        let local_branches = repo.branches(Some(git2::BranchType::Local))?;
        let remote_branches = repo.branches(Some(git2::BranchType::Remote))?;

        for local_branch in local_branches {
            let local_branch = local_branch?;
            let local_branch_name = local_branch.0.name()?;
            if let Some(local_branch_name) = local_branch_name {
                local_branch_names.push(String::from(local_branch_name));
                // 可以使用这个函数来获取上游的分支
                // pub fn upstream(&self) -> Result<Branch<'repo>, Error>
                // Return the reference supporting the remote tracking branch, given a local branch reference.
            }
        }

        for remote_branch in remote_branches {
            let remote_branch = remote_branch?;
            let remote_branch_name = remote_branch.0.name()?;
            if let Some(remote_branch_name) = remote_branch_name {
                let temp_split: Vec<&str> = remote_branch_name.split('/').collect();

                let remote_name = temp_split.first();
                let branch_name = temp_split.get(1);

                if let (Some(remote_name), Some(branch_name)) = (remote_name, branch_name) {
                    // res.push((String::from(remote_name), ))
                    let remote_branches =
                        res.entry(String::from(*remote_name)).or_insert(Vec::new());
                    remote_branches.push(String::from(*branch_name));
                }
            }
        }

        Ok(res)
    }

    pub fn get_head_branch_remote_name(repo: &Repository) -> BDEResult<Option<String>> {
        let now_head = repo.head()?;
        if !now_head.is_branch() {
            return Err(ba_error("the now head is not branch"));
        }

        let head_name = now_head.shorthand();
        if let Some(head_name) = head_name {
            let head_branch = repo.find_branch(head_name, git2::BranchType::Local)?;
            let remote_branch = head_branch.upstream()?;

            let remote_name = GitRepo::get_remote_branch_remote_name(&remote_branch)?;
            Ok(remote_name)
        } else {
            Ok(None)
        }
    }

    pub fn get_head_branch_remote_branch(
        repo: &Repository,
    ) -> BDEResult<Option<(String, String, Vec<String>)>> {
        let now_head = repo.head()?;
        if !now_head.is_branch() {
            return Err(ba_error("the now head is not branch"));
        }

        let all_remote_branch_names = GitRepo::get_all_remote_and_branchs(repo)?;

        let head_name = now_head.shorthand();
        if let Some(head_name) = head_name {
            let head_branch = repo.find_branch(head_name, git2::BranchType::Local)?;
            let remote_branch = head_branch.upstream()?;

            let local_branch_name = head_branch.name()?;

            let remote_name = GitRepo::get_remote_branch_remote_name(&remote_branch)?;
            if let (Some(local_branch_name), Some(remote_name)) = (local_branch_name, remote_name) {
                let remote_branchs = all_remote_branch_names.get(&remote_name);
                if let Some(remote_branchs) = remote_branchs {
                    return Ok(Some((
                        remote_name,
                        String::from(local_branch_name),
                        remote_branchs.clone(),
                    )));
                }
            }
        }

        Ok(None)
    }

    pub async fn test_remote_connect(path: &Path, remote_name: &str) -> bool {
        let command = format!("cd {} && git remote show {}", path.display(), remote_name);

        match run_command_timeout_no(&command, 5).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn get_head_remote_status(repo: &Repository) -> BDEResult<Option<(bool, bool)>> {
        let head_remote = GitRepo::get_head_branch_remote_branch(repo)?;
        if let Some(head_remote) = head_remote {
            let remote_name = head_remote.0;

            // 获取远程分支最新动态
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(git_credentials_callback);

            let mut opts = git2::FetchOptions::new();
            opts.remote_callbacks(callbacks);

            let mut remote = repo.find_remote(&remote_name)?;
            remote.fetch(&["master"], Some(&mut opts), None)?;
            // 获取差异
            let local_branch_ref_name = format!("refs/heads/{}", head_remote.1.clone());
            let remote_branch_ref_name =
                format!("refs/remotes/{}/{}", remote_name, head_remote.1.clone());
            // 获取本地分支和远程分支的引用
            let local_branch_ref = repo.find_reference(&local_branch_ref_name)?;
            let remote_branch_ref = repo.find_reference(&remote_branch_ref_name)?;

            // 获取本地分支和远程分支的 commit id
            let local_commit_id = local_branch_ref.target();
            let remote_commit_id = remote_branch_ref.target();

            if let (Some(local_commit_id), Some(remote_commit_id)) =
                (local_commit_id, remote_commit_id)
            {
                // 获取本地分支相对于远程分支的差异
                let mut walk = repo.revwalk()?;
                walk.push(local_commit_id)?;
                walk.hide(remote_commit_id)?;

                let has_local_commits = match walk.next() {
                    Some(res) => res.is_ok(),
                    None => false,
                };

                // 获取远程分支相对于本地分支的差异
                walk.reset()?;
                walk.push(remote_commit_id)?;
                walk.hide(local_commit_id)?;

                let has_remote_commits = match walk.next() {
                    Some(res) => res.is_ok(),
                    None => false,
                };

                // pull, push
                Ok(Some((has_remote_commits, has_local_commits)))
            } else {
                Err(ba_error("无法获取本地分支和远程分支的 commit id"))
            }
        } else {
            Ok(None)
        }
    }

    pub fn get_repo_status(
        repo: &Repository,
        remote_connectp: Option<bool>,
    ) -> BDEResult<GitStatus> {
        let mut new_status = GitStatus::Clean;

        let state = repo.state();

        // println!("repo status: {:?}", state);
        let statuses = repo.statuses(None)?;

        let mut need_commit = false;
        for status in statuses.iter() {
            if let Some(_) = status.path() {
                if status.status() != git2::Status::IGNORED {
                    // println!("file path: {}, state: {:?}", path, status.status());
                    need_commit = true;
                    break;
                }
            }
        }

        if !need_commit {
            if let Some(remote_connectp) = remote_connectp {
                if remote_connectp {
                    let remote_need_pull_push = GitRepo::get_head_remote_status(repo)?;
                    if let Some((need_pull, need_push)) = remote_need_pull_push {
                        if need_pull {
                            new_status = GitStatus::NeedPull;
                        }

                        if need_pull {
                            new_status = GitStatus::NeedPush;
                        }
                    }
                }
            }
        } else {
            new_status = GitStatus::NeedCommit;
        }

        Ok(new_status)
    }

fn git_credentials_callback(
    _url: &str,
    username: Option<&str>,
    _cred_type: CredentialType,
) -> Result<Cred, Error> {
    let public = PathBuf::from("/home/lizqwer/.ssh/id_rsa.pub");
    let private = PathBuf::from("/home/lizqwer/.ssh/id_rsa");

    // let password = env::var("GIT_PASSWORD").ok();

    let cred = Cred::ssh_key(
        username.unwrap(),
        Some(public.as_path()),
        private.as_path(),
        None,
    );

    Ok(cred?)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::utils::BDEResult;

    #[test]
    fn test_repo_status() -> BDEResult<()> {
        // let repo = Repository::open("/home/lizqwer/.emacs.d")?;
        // let repo = Repository::open("/home/lizqwer/MyProject/home_movies_tool")?;
        // let repo = Repository::open("/home/lizqwer/MyProject/git_manager")?;
        let repo = Repository::open("/home/lizqwer/quicklisp/local-projects/tel-bot/")?;

        // GitRepo::get_repo_status(&repo)?;

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_get_remote_branch() -> BDEResult<()> {
        // let repo = Repository::open("/home/lizqwer/MyProject/home_movies_tool")?;
        let repo = Repository::open("/home/lizqwer/.emacs.d")?;

        let remote = GitRepo::get_head_branch_remote_branch(&repo)?;

        if let Some(remote) = remote {
            println!("head remote name: {}", remote.0);
            println!("head remote name: {}", remote.1);
            println!("head local: {}", remote.1);

            for branch in remote.2 {
                println!("head remote branch: {}", branch);
            }
        }

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_repo() -> BDEResult<()> {
        // let repo = Repository::open("/home/lizqwer/MyProject/home_movies_tool/")?;
        let repo = Repository::open("/home/lizqwer/.emacs.d/")?;

        let remotes = repo.remotes()?;

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(git_credentials_callback);

        let mut opts = git2::FetchOptions::new();
        opts.remote_callbacks(callbacks);
        // opts.download_tags(git2::AutotagOption::All);

        for remote_name in remotes.iter() {
            println!("remote: {:?}", remote_name);
            if let Some(remote_name) = remote_name {
                let mut remote = repo.find_remote(remote_name)?;
                // remote.fetch(&["master"], Some(&mut opts), None)?;
            }
        }

        let local_branches = repo.branches(Some(git2::BranchType::Local))?;
        let remote_branches = repo.branches(Some(git2::BranchType::Remote))?;

        for local_branch in local_branches {
            let local_branch = local_branch?;
            let local_branch_name = local_branch.0.name()?;

            println!("local branch: {:?}", local_branch_name);
        }

        for remote_branch in remote_branches {
            let remote_branch = remote_branch?;
            let remote_branch_name = remote_branch.0.name()?;

            println!("remote branch: {:?}", remote_branch_name);
        }

        let now_head = repo.head()?;
        let head_name = now_head.shorthand();

        println!(
            "({})now branch: {:?}",
            if now_head.is_branch() { "local" } else { "no" },
            head_name
        );

        Ok(())
    }
}
