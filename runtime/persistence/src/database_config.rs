use std::env;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::PathBuf;

const MAX_DATABASE_URL_BYTES: usize = 2_048;

/// A redacted database-configuration failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatabaseConfigurationError {
    Missing,
    ConflictingSources,
    UnreadableFile,
    InvalidEncoding,
    InvalidLength,
    InvalidShape,
}

impl Display for DatabaseConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Missing => "DATABASE_URL or DATABASE_URL_FILE is required",
            Self::ConflictingSources => {
                "DATABASE_URL and DATABASE_URL_FILE cannot both be configured"
            }
            Self::UnreadableFile => "DATABASE_URL_FILE could not be read",
            Self::InvalidEncoding => "database URL configuration is not valid UTF-8",
            Self::InvalidLength => "database URL configuration has an invalid length",
            Self::InvalidShape => "database URL configuration has an invalid shape",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for DatabaseConfigurationError {}

/// Loads one credential-bearing `PostgreSQL` URL without retaining a committed
/// fallback or exposing its contents in configuration errors.
///
/// # Errors
///
/// Returns a redacted error for missing/conflicting sources, unreadable files,
/// invalid UTF-8, an empty/oversized value, line breaks, or a non-PostgreSQL
/// URL prefix.
pub fn database_url_from_environment() -> Result<String, DatabaseConfigurationError> {
    database_url_from_sources(
        env::var_os("DATABASE_URL").map(|value| value.to_string_lossy().into_owned()),
        env::var_os("DATABASE_URL_FILE").map(PathBuf::from),
    )
}

fn database_url_from_sources(
    direct: Option<String>,
    file: Option<PathBuf>,
) -> Result<String, DatabaseConfigurationError> {
    let bytes = match (direct, file) {
        (Some(_), Some(_)) => return Err(DatabaseConfigurationError::ConflictingSources),
        (None, None) => return Err(DatabaseConfigurationError::Missing),
        (Some(value), None) => value.into_bytes(),
        (None, Some(path)) => {
            fs::read(path).map_err(|_| DatabaseConfigurationError::UnreadableFile)?
        }
    };
    parse_database_url(bytes)
}

fn parse_database_url(mut bytes: Vec<u8>) -> Result<String, DatabaseConfigurationError> {
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
    if bytes.is_empty() || bytes.len() > MAX_DATABASE_URL_BYTES {
        return Err(DatabaseConfigurationError::InvalidLength);
    }
    if bytes.contains(&b'\n') || bytes.contains(&b'\r') || bytes.contains(&0) {
        return Err(DatabaseConfigurationError::InvalidShape);
    }
    let value =
        String::from_utf8(bytes).map_err(|_| DatabaseConfigurationError::InvalidEncoding)?;
    if !(value.starts_with("postgres://") || value.starts_with("postgresql://")) {
        return Err(DatabaseConfigurationError::InvalidShape);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        database_url_from_sources, parse_database_url, DatabaseConfigurationError,
        MAX_DATABASE_URL_BYTES,
    };

    #[test]
    fn accepts_exactly_one_redacted_database_url_source() {
        let direct = database_url_from_sources(
            Some("postgres://runtime:fixture@127.0.0.1/revenant".to_owned()),
            None,
        )
        .expect("direct URL should pass");
        assert!(direct.starts_with("postgres://runtime:"));
        assert_eq!(
            database_url_from_sources(Some(direct), Some(PathBuf::from("ignored"))),
            Err(DatabaseConfigurationError::ConflictingSources)
        );
        assert_eq!(
            database_url_from_sources(None, None),
            Err(DatabaseConfigurationError::Missing)
        );
    }

    #[test]
    fn trims_one_terminal_newline_and_rejects_unsafe_shapes() {
        assert_eq!(
            parse_database_url(b"postgresql://runtime:fixture@postgres/revenant\r\n".to_vec())
                .expect("terminal CRLF should pass"),
            "postgresql://runtime:fixture@postgres/revenant"
        );
        for bytes in [
            b"".to_vec(),
            b"http://runtime:fixture@postgres/revenant".to_vec(),
            b"postgres://runtime:fixture@postgres/a\nb".to_vec(),
            vec![b'x'; MAX_DATABASE_URL_BYTES + 1],
        ] {
            assert!(parse_database_url(bytes).is_err());
        }
    }

    #[test]
    fn errors_never_render_supplied_secret_material() {
        let marker = "m30-do-not-render-fixture";
        let error = parse_database_url(format!("http://{marker}").into_bytes())
            .expect_err("wrong scheme must reject");
        assert!(!error.to_string().contains(marker));
    }
}
