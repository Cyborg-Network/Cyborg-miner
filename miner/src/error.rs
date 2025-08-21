use derive_more::From;

/// A type alias for a `Result` with the custom error enum `Error`.
pub type Result<T> = core::result::Result<T, Error>;

/// The custom error enum for the cyborg worker. This error enum covers all of the error variants that can occur,
/// enabling all errors to be handled with the `?` operator, but not preventing handling the errors more precisely.
#[derive(Debug, From)]
pub enum Error {
    #[from]
    Custom(String),

    // -- Externals
    #[from]
    Io(std::io::Error),

    #[from]
    Serialization(serde_json::Error),

    #[from]
    Reqwest(reqwest::Error),

    #[from]
    Subxt(subxt::Error),

    #[from]
    Conversion(std::string::FromUtf8Error),

    #[from]
    Cess(cess_rust_sdk::core::Error),

    #[from]
    ReqwestToStr(reqwest::header::ToStrError),
    
    #[from]
    ReqwestParseInt(std::num::ParseIntError), 

    #[from]
    Bollard(bollard::errors::Error),
}

impl Error {
    pub fn custom(val: impl std::fmt::Display) -> Self {
        Self::Custom(val.to_string())
    }

    pub fn parachain_client_not_intitialized() -> Self {
        Self::Custom("Parachain client not initialized".to_string())
    }

    pub fn identity_not_initialized() -> Self {
        Self::Custom("Identity not initialized".to_string())
    }

    pub fn config_paths_not_initialized() -> Self {
        Self::Custom("Config paths not initialized".to_string())
    }

    pub fn no_current_task() -> Self {
        Self::Custom("The miner does not have a running task".to_string())
    }

    pub fn cess_gateway_not_initialized() -> Self {
        Self::Custom("CESS gateway not initialized".to_string())
    }
}

impl From<&str> for Error {
    fn from(val: &str) -> Self {
        Self::Custom(val.to_string())
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
