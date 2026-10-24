//! Error type and exit-code classification for Omoi.

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Every failure Omoi classifies for its console contract.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration: {0}")]
    Config(String),

    #[error("I/O during {context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("database version {found} is newer than supported {supported}")]
    SchemaTooNew { found: u32, supported: u32 },

    #[error("unsupported database schema: {0}")]
    Schema(String),

    #[error("writer busy")]
    Busy,

    #[error("profile {source_id}: {reason}")]
    Source { source_id: String, reason: String },

    #[error("export: {0}")]
    Export(String),

    #[error("unsupported: {0}")]
    Unsupported(&'static str),

    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

impl Error {
    /// Map the error to the process exit code defined by the CLI contract.
    ///
    /// Only the four distinctions exercised by the contract are special:
    /// configuration is 2, a busy writer is 4, an unsupported platform
    /// operation is 5, and every other fatal failure is 1.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Config(_) => 2,
            Self::Busy => 4,
            Self::Unsupported(_) => 5,
            Self::Io { .. }
            | Self::Db(_)
            | Self::SchemaTooNew { .. }
            | Self::Schema(_)
            | Self::Source { .. }
            | Self::Export(_)
            | Self::NotImplemented(_) => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn fatal_errors_are_exit_one() {
        assert_eq!(
            Error::SchemaTooNew {
                found: 2,
                supported: 1
            }
            .exit_code(),
            1
        );
        assert_eq!(Error::NotImplemented("cli").exit_code(), 1);
    }
}
