use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [argument] if argument == "--healthcheck" => return healthcheck(),
        [argument] if argument == "--migrate-only" => {
            return if revenant_gateway::migrate_database().is_ok() {
                ExitCode::SUCCESS
            } else {
                eprintln!("{{\"event\":\"session_command_failed\",\"category\":\"persistence\"}}");
                ExitCode::FAILURE
            };
        }
        [] => {}
        _ => {
            eprintln!(
                "{{\"event\":\"session_command_failed\",\"category\":\"invalid_arguments\"}}"
            );
            return ExitCode::FAILURE;
        }
    }

    let bind_addr = env::var("REVENANT_BIND_ADDR")
        .unwrap_or_else(|_| revenant_gateway::DEFAULT_BIND_ADDR.to_owned());
    let game_addr = env::var("REVENANT_GAME_ADDR")
        .unwrap_or_else(|_| revenant_gateway::DEFAULT_GAME_ADDR.to_owned());

    if revenant_gateway::run(&bind_addr, &game_addr).is_ok() {
        ExitCode::SUCCESS
    } else {
        eprintln!("{{\"event\":\"session_command_failed\",\"category\":\"internal\"}}");
        ExitCode::FAILURE
    }
}

fn healthcheck() -> ExitCode {
    let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080") else {
        return ExitCode::FAILURE;
    };
    if stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return ExitCode::FAILURE;
    }

    let mut response = String::new();
    if stream.read_to_string(&mut response).is_ok() && response.starts_with("HTTP/1.1 200 OK") {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
