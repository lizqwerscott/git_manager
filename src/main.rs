use std::path::Path;

use git_manager::search_all_git_repo;

fn main() {
    let res = search_all_git_repo(Path::new("~/")).unwrap();
    println!("res: {}", res.len());
}
