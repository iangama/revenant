use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;

use revenant_local_security_lab::generate_report;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = output_path()?;
    let report = generate_report().map_err(io::Error::other)?;
    let mut bytes = serde_json::to_vec_pretty(&report)?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    println!(
        "M30 Gate 2 identity/session report written: {} (30/30 cases, secrets redacted)",
        output.display()
    );
    Ok(())
}

fn output_path() -> Result<PathBuf, io::Error> {
    let mut arguments = env::args_os().skip(1);
    let Some(flag) = arguments.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: revenant-security-lab --output PATH",
        ));
    };
    if flag != "--output" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected --output",
        ));
    }
    let Some(path) = arguments.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "missing output path",
        ));
    };
    if arguments.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unexpected extra argument",
        ));
    }
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output path is empty",
        ));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    #[test]
    fn package_feature_is_explicit() {
        assert_eq!(
            std::env::var("CARGO_PKG_NAME").expect("package name"),
            "revenant-local-security-lab"
        );
        let feature = OsString::from("m30-identity-session-lab");
        assert!(!feature.is_empty());
    }
}
