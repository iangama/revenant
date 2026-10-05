use std::env;
use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    if run().is_ok() {
        ExitCode::SUCCESS
    } else {
        eprintln!("{{\"event\":\"session_command_failed\",\"category\":\"internal\"}}");
        ExitCode::FAILURE
    }
}

fn run() -> io::Result<()> {
    let bind_addr = env::var("REVENANT_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:18029".to_owned());
    let game_addr = env::var("REVENANT_GAME_ADDR").unwrap_or_else(|_| "127.0.0.1:17029".to_owned());
    let seed_text = env::var("REVENANT_M27_MATRIX_SEEDS")
        .map_err(|_| io::Error::other("REVENANT_M27_MATRIX_SEEDS is required"))?;
    let seeds = parse_seed_queue(&seed_text)?;
    revenant_gateway::run_m27_matrix(&bind_addr, &game_addr, seeds)
}

fn parse_seed_queue(value: &str) -> io::Result<Vec<u64>> {
    if value.is_empty() {
        return Err(io::Error::other("M27 matrix seed queue is empty"));
    }
    value
        .split(',')
        .map(|seed| {
            if seed.is_empty() || !seed.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(io::Error::other(
                    "M27 matrix seeds must be comma-separated decimal integers",
                ));
            }
            seed.parse::<u64>()
                .map_err(|error| io::Error::other(format!("invalid M27 matrix seed: {error}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_seed_queue;

    #[test]
    fn matrix_seed_queue_parser_is_strict() {
        assert_eq!(parse_seed_queue("0,1,2").expect("valid queue"), [0, 1, 2]);
        for invalid in ["", "0,", ",0", "0, 1", "-1", "one"] {
            assert!(parse_seed_queue(invalid).is_err(), "{invalid}");
        }
    }
}
