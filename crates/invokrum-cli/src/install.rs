use std::ffi::OsString;

use invokrum_install_delivery::{ErrorKind, InstallError};

use crate::{CliError, Execution};

pub(crate) fn is_install_command(arguments: &[OsString]) -> bool {
    invokrum_install_delivery::is_install_command(arguments)
}

pub(crate) fn execute(arguments: &[OsString]) -> Result<Execution, CliError> {
    invokrum_install_delivery::execute(arguments)
        .map(|execution| Execution::success(execution.into_stdout()))
        .map_err(|error| map_error(&error))
}

fn map_error(error: &InstallError) -> CliError {
    match error.kind() {
        ErrorKind::Usage => CliError::usage(error.message()),
        ErrorKind::Input => CliError::input(error.message()),
        ErrorKind::Validation => CliError::validation(error.message()),
        ErrorKind::Output => CliError::output(error.message()),
        ErrorKind::Internal => CliError::internal(error.message()),
    }
}
