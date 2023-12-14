use std::path::Path;

use git_manager::search_all_git_repo;

#[tokio::main]
async fn main() {
    let test_path_1 = "~/";
    let test_path_2 = "~/AndroidStudioProjects/";
    
    let search_path = Path::new(test_path_1);

    let mut repos = search_all_git_repo(search_path).await.unwrap();

    println!("res: {}", repos.len());
    println!("{:16}: {:10}", "仓库名字", "状态");
    for repo in repos.iter_mut() {
        println!("{}", repo);
    }
}
