use std::{fmt::Display, process::Command};

#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("Failed to execute the `{0}` process")]
    ExecuteProcessError(#[from] std::io::Error),
    #[error("`{0}` failed to execute successfully")]
    CommandExecutionError(String),
}

pub struct CommandStructure<'a> {
    command: &'a str,
    arg: &'a str,
}

impl<'a> CommandStructure<'a> {
    pub fn new(command: &'a str, arg: &'a str) -> Self {
        Self { command, arg }
    }

    pub fn execute_command(&self) -> Result<(), CommandError> {
        let status = Command::new("sh")
            .arg("-c")
            .arg(format!("{} {}", self.command, self.arg))
            .status()
            .map_err(CommandError::ExecuteProcessError)?;

        if status.success() {
            println!("{} Successfully executed", self);
            Ok(())
        } else {
            Err(CommandError::CommandExecutionError(format!("{}", self)))
        }
    }
}

impl<'a> Display for CommandStructure<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command: {}, with arg: {}\n", self.command, self.arg)
    }
}
