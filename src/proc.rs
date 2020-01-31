use std::io;
use std::io::BufRead;
use std::process::{Command, ExitStatus};
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum CommandError {
    Io(Box<io::Error>),
    Stderr(String),
    De(serde_json::Error),
    Encoding(FromUtf8Error),
    ExitStatus(ExitStatus),
}

impl From<io::Error> for CommandError {
    fn from(e: io::Error) -> Self {
        CommandError::Io(Box::new(e))
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(e: serde_json::Error) -> Self {
        CommandError::De(e)
    }
}

pub fn run_cmd<F, T>(c: &mut Command, f: F) -> Result<T, CommandError>
where
    F: FnOnce(Vec<u8>) -> T,
{
    let output = c.output().map_err(Box::new).map_err(CommandError::Io)?;

    if !output.status.success() {
        return Err(CommandError::ExitStatus(output.status));
    }

    if !output.stderr.is_empty() {
        return Err(CommandError::Stderr(
            String::from_utf8(output.stderr).map_err(CommandError::Encoding)?,
        ));
    }

    Ok(f(output.stdout))
}

pub fn run_cmd_stdout(c: &mut Command) -> Result<String, CommandError> {
    run_cmd(c, String::from_utf8)?.map_err(CommandError::Encoding)
}

pub fn run_cmd_stdout_lines_capacity(
    c: &mut Command,
    lines_hint: usize,
) -> Result<Vec<String>, CommandError> {
    let mut ret = Vec::with_capacity(lines_hint);
    ret.extend(run_cmd(c, |stdout| {
        stdout
            .lines()
            .collect::<Result<_, io::Error>>()
            .map_err(Into::<CommandError>::into)
    })?);
    Ok(ret)
}

pub fn run_cmd_stdout_lines(c: &mut Command) -> Result<Vec<String>, CommandError> {
    run_cmd_stdout_lines_capacity(c, 64)
}
