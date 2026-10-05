use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::net::TcpStream;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use revenant_protocol::{
    read_message, write_message, AuthRequest, CharacterListRequest, ClientHello, ClientMessage,
    ModuleStateRequest, ServerMessage, WorldJoinRequest, MAX_FRAME_SIZE, PROTOCOL_VERSION,
};

const IO_TIMEOUT: Duration = Duration::from_secs(18);
const CLOSE_PROBE_TIMEOUT: Duration = Duration::from_millis(20);

pub(super) fn run(mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    let address = env::var("REVENANT_GAME_ADDR").unwrap_or_else(|_| "127.0.0.1:7000".to_owned());
    match mode {
        "exact-frame" => exact_frame(&address),
        "oversize-frame" => oversize_frame(&address),
        "incomplete-prefix" => incomplete_frame(&address, true),
        "incomplete-body" => incomplete_frame(&address, false),
        "invalid-messagepack" => rejected_payload(&address, &[0xc1], "invalid_messagepack"),
        "wrong-order" => wrong_order(&address),
        "unsupported-version" => unsupported_version(&address),
        "unauthenticated-boundary" => unauthenticated_boundary(&address),
        "global-boundary" => global_boundary(&address),
        "rate-burst" => rate_burst(&address),
        "recovery-hold" => recovery_hold(&address),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown M30 security probe mode",
        )
        .into()),
    }
}

fn stream(address: &str) -> io::Result<TcpStream> {
    let stream = TcpStream::connect(address)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    stream.set_nodelay(true)?;
    Ok(stream)
}

fn exact_frame(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut name_length = MAX_FRAME_SIZE.saturating_sub(64);
    let payload = loop {
        let candidate = rmp_serde::to_vec_named(&ClientMessage::ClientHello(ClientHello {
            protocol_version: PROTOCOL_VERSION,
            client_name: "x".repeat(name_length),
            client_build: "m30".to_owned(),
            content_revision: None,
        }))?;
        match candidate.len().cmp(&MAX_FRAME_SIZE) {
            std::cmp::Ordering::Equal => break candidate,
            std::cmp::Ordering::Less => name_length += MAX_FRAME_SIZE - candidate.len(),
            std::cmp::Ordering::Greater => name_length -= candidate.len() - MAX_FRAME_SIZE,
        }
    };
    let mut connection = stream(address)?;
    write_payload(&mut connection, &payload)?;
    let ServerMessage::ServerHello(hello) = read_message(&mut connection)? else {
        return Err(io::Error::other("exact frame returned the wrong response").into());
    };
    if !hello.accepted {
        return Err(io::Error::other("exact frame was not decoded and accepted").into());
    }
    println!(
        "{{\"probe\":\"exact_frame\",\"bytes\":{MAX_FRAME_SIZE},\"disposition\":\"decoded\"}}"
    );
    Ok(())
}

fn oversize_frame(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = stream(address)?;
    let announced = u32::try_from(MAX_FRAME_SIZE + 1)?;
    connection.write_all(&announced.to_be_bytes())?;
    connection.flush()?;
    if !wait_for_close(&connection)? {
        return Err(io::Error::other("oversize announcement remained connected").into());
    }
    println!(
        "{{\"probe\":\"oversize_frame\",\"bytes\":{},\"disposition\":\"closed\"}}",
        MAX_FRAME_SIZE + 1
    );
    Ok(())
}

fn incomplete_frame(address: &str, prefix_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = stream(address)?;
    if prefix_only {
        connection.write_all(&[0, 0])?;
    } else {
        connection.write_all(&100_u32.to_be_bytes())?;
        connection.write_all(&[0x81])?;
    }
    connection.flush()?;
    let started = Instant::now();
    if !wait_for_close(&connection)? {
        return Err(io::Error::other("incomplete frame did not close").into());
    }
    let elapsed = started.elapsed();
    if elapsed < Duration::from_secs(14) || elapsed > Duration::from_millis(17_500) {
        return Err(io::Error::other("incomplete frame closed outside handshake boundary").into());
    }
    let label = if prefix_only {
        "incomplete_prefix"
    } else {
        "incomplete_body"
    };
    println!(
        "{{\"probe\":\"{label}\",\"duration_ms\":{},\"disposition\":\"closed\"}}",
        elapsed.as_millis()
    );
    Ok(())
}

fn rejected_payload(
    address: &str,
    payload: &[u8],
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = stream(address)?;
    write_payload(&mut connection, payload)?;
    if !wait_for_close(&connection)? {
        return Err(io::Error::other("rejected payload remained connected").into());
    }
    println!("{{\"probe\":\"{label}\",\"disposition\":\"closed\"}}");
    Ok(())
}

fn wrong_order(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let payload = rmp_serde::to_vec_named(&ClientMessage::AuthRequest(AuthRequest {
        username: "m30wrongorder".to_owned(),
    }))?;
    rejected_payload(address, &payload, "wrong_order")
}

fn unsupported_version(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = stream(address)?;
    write_message(
        &mut connection,
        &ClientMessage::ClientHello(ClientHello {
            protocol_version: PROTOCOL_VERSION + 1,
            client_name: "m30".to_owned(),
            client_build: "m30".to_owned(),
            content_revision: None,
        }),
    )?;
    let ServerMessage::ServerHello(hello) = read_message(&mut connection)? else {
        return Err(io::Error::other("unsupported version returned the wrong response").into());
    };
    if hello.accepted || !wait_for_close(&connection)? {
        return Err(io::Error::other("unsupported version did not reject and close").into());
    }
    println!("{{\"probe\":\"unsupported_version\",\"disposition\":\"rejected\"}}");
    Ok(())
}

fn unauthenticated_boundary(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut established = Vec::with_capacity(8);
    for _ in 0..8 {
        established.push(stream(address)?);
    }
    thread::sleep(Duration::from_millis(100));
    if established.iter().any(is_closed) {
        return Err(io::Error::other("one of the first eight unauthenticated peers closed").into());
    }
    let ninth = stream(address)?;
    if !wait_for_close(&ninth)? || established.iter().any(is_closed) {
        return Err(io::Error::other("ninth unauthenticated boundary was not isolated").into());
    }
    println!("{{\"probe\":\"unauthenticated_boundary\",\"accepted\":8,\"rejected\":9,\"established_retained\":true}}");
    Ok(())
}

fn global_boundary(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let warm = authenticated_connection(address, "m30global")?;
    drop(warm);
    thread::sleep(Duration::from_millis(100));

    let mut established = Vec::with_capacity(64);
    for _ in 0..64 {
        established.push(authenticated_connection(address, "m30global")?);
    }
    if established.iter().any(is_closed) {
        return Err(io::Error::other("one of the first 64 authenticated peers closed").into());
    }
    let sixty_fifth = stream(address)?;
    if !wait_for_close(&sixty_fifth)? || established.iter().any(is_closed) {
        return Err(io::Error::other("65th global boundary was not isolated").into());
    }
    if env::var_os("M30_PROBE_READY_FILE").is_some() {
        signal_ready_and_wait()?;
    }
    println!("{{\"probe\":\"global_boundary\",\"accepted\":64,\"rejected\":65,\"established_retained\":true}}");
    Ok(())
}

fn authenticated_connection(
    address: &str,
    username: &str,
) -> Result<TcpStream, Box<dyn std::error::Error>> {
    let mut connection = stream(address)?;
    write_message(
        &mut connection,
        &ClientMessage::ClientHello(ClientHello {
            protocol_version: PROTOCOL_VERSION,
            client_name: "m30".to_owned(),
            client_build: "m30".to_owned(),
            content_revision: None,
        }),
    )?;
    let ServerMessage::ServerHello(hello) = read_message(&mut connection)? else {
        return Err(io::Error::other("authentication fixture received wrong hello").into());
    };
    if !hello.accepted {
        return Err(io::Error::other("authentication fixture hello was rejected").into());
    }
    write_message(
        &mut connection,
        &ClientMessage::AuthRequest(AuthRequest {
            username: username.to_owned(),
        }),
    )?;
    let ServerMessage::AuthResponse(response) = read_message(&mut connection)? else {
        return Err(io::Error::other("authentication fixture received wrong auth response").into());
    };
    if !response.authenticated {
        return Err(io::Error::other("authentication fixture was rejected").into());
    }
    Ok(connection)
}

fn recovery_hold(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let username = env::var("REVENANT_BOT_USERNAME")?;
    if !username.starts_with("m30") || !address.parse::<std::net::SocketAddr>()?.ip().is_loopback()
    {
        return Err(
            io::Error::other("recovery fixture requires a synthetic local identity").into(),
        );
    }
    let mut connection = authenticated_connection(address, &username)?;
    write_message(
        &mut connection,
        &ClientMessage::CharacterListRequest(CharacterListRequest {}),
    )?;
    let ServerMessage::CharacterListResponse(characters) = read_message(&mut connection)? else {
        return Err(io::Error::other("recovery fixture character response diverged").into());
    };
    let character_id = characters
        .characters
        .first()
        .ok_or_else(|| io::Error::other("recovery fixture has no character"))?
        .character_id
        .clone();
    write_message(
        &mut connection,
        &ClientMessage::WorldJoinRequest(WorldJoinRequest { character_id }),
    )?;
    let ServerMessage::WorldJoinResponse(world) = read_message(&mut connection)? else {
        return Err(io::Error::other("recovery fixture world response diverged").into());
    };
    if !world.accepted {
        return Err(io::Error::other("recovery fixture admission rejected").into());
    }
    let mut started = false;
    let mut spawned = false;
    for _ in 0..16 {
        match read_message::<_, ServerMessage>(&mut connection)? {
            ServerMessage::ActivityStart(_) => started = true,
            ServerMessage::ActorSpawn(actor) if actor.actor_kind == "enemy" => {
                spawned = true;
                break;
            }
            _ => {}
        }
    }
    if !started || !spawned {
        return Err(io::Error::other("recovery fixture activity did not start").into());
    }
    signal_ready_and_wait()?;
    // The driver releases this only after the named fixture service has stopped.
    // An equipment change requires persistence before any success projection.
    let _ = write_message(
        &mut connection,
        &ClientMessage::EquipIntent(revenant_protocol::EquipIntent {
            item_id: "arc_sidearm".to_owned(),
        }),
    );
    loop {
        match read_message::<_, ServerMessage>(&mut connection) {
            Ok(
                ServerMessage::ActivityComplete(_)
                | ServerMessage::LootGranted(_)
                | ServerMessage::ProgressionGranted(_)
                | ServerMessage::EquipmentChanged(_),
            ) => {
                return Err(
                    io::Error::other("faulted fixture projected a committed result").into(),
                );
            }
            Ok(_) => {}
            Err(_) => {
                if !wait_for_close(&connection)? {
                    return Err(
                        io::Error::other("faulted fixture did not close its connection").into(),
                    );
                }
                break;
            }
        }
    }
    println!("{{\"probe\":\"recovery_hold\",\"disposition\":\"closed_without_success\"}}");
    Ok(())
}

fn rate_burst(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut connection = authenticated_connection(address, "m30rate")?;
    write_message(
        &mut connection,
        &ClientMessage::CharacterListRequest(CharacterListRequest {}),
    )?;
    let ServerMessage::CharacterListResponse(response) = read_message(&mut connection)? else {
        return Err(io::Error::other("rate fixture received wrong character response").into());
    };
    let character_id = response
        .characters
        .first()
        .ok_or_else(|| io::Error::other("rate fixture has no character"))?
        .character_id
        .clone();
    write_message(
        &mut connection,
        &ClientMessage::WorldJoinRequest(WorldJoinRequest { character_id }),
    )?;
    let ServerMessage::WorldJoinResponse(world) = read_message(&mut connection)? else {
        return Err(io::Error::other("rate fixture received wrong world response").into());
    };
    if !world.accepted {
        return Err(io::Error::other("rate fixture world join was rejected").into());
    }
    for expected in ["inventory", "progression", "equipment"] {
        let response: ServerMessage = read_message(&mut connection)?;
        let matches_expected = matches!(
            (expected, response),
            ("inventory", ServerMessage::InventorySnapshot(_))
                | ("progression", ServerMessage::ProgressionSnapshot(_))
                | ("equipment", ServerMessage::EquipmentSnapshot(_))
        );
        if !matches_expected {
            return Err(io::Error::other("rate fixture snapshot order diverged").into());
        }
    }

    let peer = authenticated_connection(address, "m30peer")?;
    signal_ready_and_wait()?;
    // Expire handshake traffic before exercising a fresh rolling second.
    thread::sleep(Duration::from_millis(1_100));
    let started = Instant::now();
    for _ in 0..33 {
        write_message(
            &mut connection,
            &ClientMessage::ModuleStateRequest(ModuleStateRequest {}),
        )?;
    }
    let mut accepted = 0_usize;
    loop {
        match read_message::<_, ServerMessage>(&mut connection) {
            Ok(ServerMessage::ModuleSnapshot(_)) => accepted += 1,
            Ok(_) => return Err(io::Error::other("rate fixture received an extra message").into()),
            Err(_) => break,
        }
    }
    if accepted != 32 || started.elapsed() >= Duration::from_secs(1) || is_closed(&peer) {
        return Err(
            io::Error::other("rate fixture did not enforce frames 32 and 33 exactly").into(),
        );
    }
    println!(
        "{{\"probe\":\"rate_burst\",\"accepted_total\":32,\"rejected_total\":33,\"accepted_gameplay\":{accepted},\"peer_retained\":true,\"disposition\":\"offender_closed\"}}"
    );
    Ok(())
}

fn signal_ready_and_wait() -> io::Result<()> {
    let ready = env::var_os("M30_PROBE_READY_FILE")
        .ok_or_else(|| io::Error::other("M30_PROBE_READY_FILE is required"))?;
    let go = env::var_os("M30_PROBE_GO_FILE")
        .ok_or_else(|| io::Error::other("M30_PROBE_GO_FILE is required"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)?;
    file.write_all(b"ready\n")?;
    file.sync_all()?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !Path::new(&go).is_file() {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "rate fixture go timeout",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn write_payload(connection: &mut TcpStream, payload: &[u8]) -> io::Result<()> {
    let size = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "probe payload is too large"))?;
    connection.write_all(&size.to_be_bytes())?;
    connection.write_all(payload)?;
    connection.flush()
}

fn wait_for_close(connection: &TcpStream) -> io::Result<bool> {
    connection.set_read_timeout(Some(IO_TIMEOUT))?;
    let result = is_closed_result(connection);
    connection.set_read_timeout(Some(IO_TIMEOUT))?;
    result
}

fn is_closed(connection: &TcpStream) -> bool {
    connection
        .set_read_timeout(Some(CLOSE_PROBE_TIMEOUT))
        .and_then(|()| is_closed_result(connection))
        .unwrap_or(true)
}

fn is_closed_result(connection: &TcpStream) -> io::Result<bool> {
    let mut byte = [0_u8; 1];
    match connection.peek(&mut byte) {
        Ok(0) => Ok(true),
        Ok(_) => Ok(false),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) =>
        {
            Ok(false)
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionReset
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::BrokenPipe
            ) =>
        {
            Ok(true)
        }
        Err(error) => Err(error),
    }
}
