use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;

fn main() -> ExitCode {
    if env::args().any(|argument| argument == "--healthcheck") {
        return healthcheck();
    }

    let bind_addr = env::var("REVENANT_BIND_ADDR")
        .unwrap_or_else(|_| revenant_gateway::DEFAULT_BIND_ADDR.to_owned());
    let game_addr = env::var("REVENANT_GAME_ADDR")
        .unwrap_or_else(|_| revenant_gateway::DEFAULT_GAME_ADDR.to_owned());

    match revenant_gateway::run(&bind_addr, &game_addr) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "failed to start Revenant gateway on health={bind_addr} game={game_addr}: {error}"
            );
            ExitCode::FAILURE
        }
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
