use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use revenant_activities::validate_activity_file;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Arguments {
    json: bool,
    path: PathBuf,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("activity validation failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = parse_arguments(env::args().skip(1))?;
    let manifest = validate_activity_file(&arguments.path)?;
    if arguments.json {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        println!(
            "valid activity={} kind={} revision={} objectives={} triggers={} routes={} source_limit={} memory_limit={} instruction_limit={}",
            manifest.activity_id,
            manifest.activity_kind,
            manifest.authoring_revision.as_deref().unwrap_or("compatibility"),
            manifest.objectives.len(),
            manifest.triggers.len(),
            manifest.routes.len(),
            manifest.limits.source_bytes,
            manifest.limits.memory_bytes,
            manifest.limits.instructions
        );
    }
    Ok(())
}

fn parse_arguments(arguments: impl Iterator<Item = String>) -> Result<Arguments, io::Error> {
    let values = arguments.collect::<Vec<_>>();
    let (json, path) = match values.as_slice() {
        [path] if !path.starts_with('-') => (false, path),
        [option, path] if option == "--json" && !path.starts_with('-') => (true, path),
        _ => {
            return Err(io::Error::other(
                "usage: revenant-activity-validator [--json] <activity.lua>",
            ))
        }
    };
    Ok(Arguments {
        json,
        path: PathBuf::from(path),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{parse_arguments, Arguments};

    #[test]
    fn arguments_are_exact_and_bounded() {
        assert_eq!(
            parse_arguments(["activity.lua".to_owned()].into_iter()).expect("path should work"),
            Arguments {
                json: false,
                path: PathBuf::from("activity.lua")
            }
        );
        assert_eq!(
            parse_arguments(["--json".to_owned(), "activity.lua".to_owned()].into_iter())
                .expect("JSON path should work"),
            Arguments {
                json: true,
                path: PathBuf::from("activity.lua")
            }
        );
        assert!(parse_arguments(["--unknown".to_owned()].into_iter()).is_err());
        assert!(parse_arguments(std::iter::empty()).is_err());
    }
}
