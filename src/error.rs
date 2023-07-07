use std::{
    error::Error,
    fmt::Display,
    process::{ExitCode, ExitStatus},
};

/// An error representing a subprocess failure. **This should only be used when
/// we want to propagate the exit code**. Not all subprocesses warrant this
/// behavior!
#[derive(Copy, Clone, Debug)]
pub struct ExitCodeError(Option<i32>);

impl Display for ExitCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "process failed with exit code: ")?;
        match self.0 {
            Some(code) => write!(f, "{code}"),
            None => write!(f, "none"),
        }
    }
}

impl Error for ExitCodeError {}

impl From<&ExitStatus> for ExitCodeError {
    fn from(value: &ExitStatus) -> Self {
        Self(value.code())
    }
}

impl From<ExitCodeError> for ExitCode {
    fn from(value: ExitCodeError) -> Self {
        match value.0 {
            Some(code) => {
                // ExitStatus uses i32 but ExitCode uses u8, so we have to toss
                // out the value if it doesn't fit in u8
                match u8::try_from(code) {
                    Ok(code) => ExitCode::from(code),
                    Err(_) => ExitCode::FAILURE,
                }
            }
            None => ExitCode::FAILURE,
        }
    }
}
