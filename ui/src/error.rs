#[derive(Debug)]
pub enum Error {
    ArgumentParsingError(Box<dyn std::error::Error>),
    InvalidServerConfiguration(Box<dyn std::error::Error>),
    LaunchError(rocket::error::LaunchError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ArgumentParsingError(e) => write!(f, "argument parsing error: {}", e),
            Self::InvalidServerConfiguration(e) => write!(f, "invalid server configuration: {}", e),
            Self::LaunchError(e) => write!(f, "launch error: {}", e),
        }
    }
}
impl std::error::Error for Error {}
