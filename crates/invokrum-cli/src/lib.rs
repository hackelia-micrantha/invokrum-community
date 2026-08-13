//! Operator-facing delivery boundary for Invokrum.

#![forbid(unsafe_code)]

mod args;
mod command;
mod install;
mod output;
mod rpc;

use std::ffi::OsString;
use std::fmt;
use std::io::{Read, Write};

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_INPUT: i32 = 3;
pub const EXIT_VALIDATION: i32 = 4;
pub const EXIT_DRIFT: i32 = 5;
pub const EXIT_OUTPUT: i32 = 6;
pub const EXIT_INTERNAL: i32 = 7;

pub fn run(
    arguments: impl IntoIterator<Item = OsString>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    run_with_stdin(arguments, &mut std::io::empty(), stdout, stderr)
}

pub fn run_with_stdin(
    arguments: impl IntoIterator<Item = OsString>,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let arguments = arguments.into_iter().collect::<Vec<_>>();

    let execution = if install::is_install_command(&arguments) {
        install::execute(&arguments[1..])
    } else {
        let command = match args::parse(arguments) {
            Ok(command) => command,
            Err(message) if message == "help" => args::Command::Help,
            Err(message) => {
                emit_error(stderr, &CliError::usage(message));
                return EXIT_USAGE;
            }
        };

        if matches!(command, args::Command::Rpc) {
            Ok(rpc::execute(stdin))
        } else {
            command::execute(command)
        }
    };

    match execution {
        Ok(execution) => {
            if stdout.write_all(&execution.stdout).is_err() {
                emit_error(stderr, &CliError::output("failed to write stdout"));
                EXIT_OUTPUT
            } else {
                execution.code
            }
        }
        Err(error) => {
            let code = error.kind.exit_code();
            emit_error(stderr, &error);
            code
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ErrorKind {
    Usage,
    Input,
    Validation,
    Composition,
    Integrity,
    Output,
    Internal,
}

impl ErrorKind {
    const fn exit_code(self) -> i32 {
        match self {
            Self::Usage => EXIT_USAGE,
            Self::Input => EXIT_INPUT,
            Self::Validation | Self::Composition => EXIT_VALIDATION,
            Self::Integrity | Self::Internal => EXIT_INTERNAL,
            Self::Output => EXIT_OUTPUT,
        }
    }

    const fn code(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::Input => "input",
            Self::Validation => "validation",
            Self::Composition => "composition",
            Self::Integrity => "integrity",
            Self::Output => "output",
            Self::Internal => "internal",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CliError {
    kind: ErrorKind,
    message: String,
}

impl CliError {
    fn new(kind: ErrorKind, message: impl fmt::Display) -> Self {
        Self {
            kind,
            message: message.to_string(),
        }
    }

    pub(crate) fn usage(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Usage, message)
    }

    pub(crate) fn input(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Input, message)
    }

    pub(crate) fn validation(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Validation, message)
    }

    pub(crate) fn composition(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Composition, message)
    }

    pub(crate) fn integrity(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Integrity, message)
    }

    pub(crate) fn output(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Output, message)
    }

    pub(crate) fn internal(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Internal, message)
    }
}

pub(crate) struct Execution {
    pub code: i32,
    pub stdout: Vec<u8>,
}

impl Execution {
    pub(crate) fn success(stdout: Vec<u8>) -> Self {
        Self {
            code: EXIT_SUCCESS,
            stdout,
        }
    }
}

fn emit_error(stderr: &mut dyn Write, error: &CliError) {
    let message = escape_human(&error.message);
    let _ = writeln!(stderr, "error[{}]: {message}", error.kind.code());
}

pub(crate) fn escape_human(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{{{:x}}}", u32::from(character)));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::escape_human;

    #[test]
    fn human_output_visibly_encodes_control_characters() {
        assert_eq!(
            escape_human("line\nnext\t\u{1b}\\"),
            "line\\nnext\\t\\u{1b}\\\\"
        );
    }
}
