use std::path::Path;

use git_manager::search_all_git_repo;

#[tokio::main]
async fn main() {
    let mut repos = search_all_git_repo(Path::new("~/AndroidStudioProjects/")).unwrap();
    // let mut repos = search_all_git_repo(Path::new("~/")).unwrap();

    println!("res: {}", repos.len());
    for repo in repos.iter_mut() {
        println!("start refresh commit: {}", repo.name);
        match repo.refresh_status().await {
            Ok(_) => {}
            Err(err) => {
                println!("{}: refresh status run err: {}", repo.name, err);
            }
        }
        repo.print_status();
    }
}
