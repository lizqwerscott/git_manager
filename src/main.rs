use git_manager::run;
use git_manager::utils::BDEResult;

#[tokio::main]
async fn main() -> BDEResult<()> {
    run().await?;
    Ok(())
}
