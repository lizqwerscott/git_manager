use std::process::{self, Stdio};
use tokio::signal::ctrl_c;
use tokio::time::timeout;
use tokio::time::Duration;

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

pub fn run_command(command: &str) -> BDEResult<String> {
    match process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .output()
    {
        Ok(output) => Ok(String::from_utf8(output.stdout).unwrap()),
        Err(error) => Err(ba_error(format!("执行命令失败: {}", error).as_mut_str())),
    }
}

pub fn run_command_no(command: &str) -> BDEResult<()> {
    process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .spawn()?;

    Ok(())
}

pub async fn run_command_timeout(command: &str, timeout_second: u64) -> BDEResult<String> {
    let timeout_duration = Duration::from_secs(timeout_second);

    let mut child = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped()) // 捕获标准输出
        .stderr(Stdio::null()) // 将标准错误重定向到空
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
                Err(format!("Command failed with exit code({}): {}", output.status, String::from_utf8(output.stdout).unwrap()).into())
            }
        }

        // Wait for Ctrl+C or timeout
        _ = timeout(timeout_duration, ctrl_c_future) => {
            child.kill().await?;
            Err(ba_error("Command timed out"))
        }
    }
}

pub async fn run_command_timeout_no(command: &str, timeout_second: u64) -> BDEResult<()> {
    let timeout_duration = Duration::from_secs(timeout_second);

    let mut child = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null()) // 标准输出重定向到空
        .stderr(Stdio::null()) // 将标准错误重定向到空
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    // Create a future that resolves when Ctrl+C is pressed
    let ctrl_c_future = ctrl_c();

    tokio::select! {
        // Wait for the command to complete
        _ = child.wait() => {
            Ok(())
            // let output = child.wait_with_output().await?;
            // if output.status.success() {
            //     // Ok(String::from_utf8(output.stdout).unwrap())
            //     Ok(())
            // } else {
            //     Err(format!("Command failed with exit code({}): {}", output.status, String::from_utf8(output.stdout).unwrap()).into())
            // }
        }

        // Wait for Ctrl+C or timeout
        _ = timeout(timeout_duration, ctrl_c_future) => {
            child.kill().await?;
            Err(ba_error("Command timed out"))
        }
    }
}


// 需要安装 xclip
pub fn copy_to_clipboard(text: &str) -> BDEResult<()> {
    let command = format!("echo '{}' | xclip -selection clipboard", text);
    run_command_no(&command)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::copy_to_clipboard;

    #[test]
    fn test_clipboard() {
        let _ = copy_to_clipboard("hello world bababa");
    }
}
