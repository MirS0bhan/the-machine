//! Fallback Shell - minimal text UI for The Machine.
//!
//! Two modes:
//!   * default / --server : acts as an MCP service on fallback-shell.sock
//!   * --console          : interactive agent console that talks to mcp-bus

use common::*;
use serde_json::Value;
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::OwnedReadHalf;
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::UnixListener;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--console") {
        return run_console().await;
    }
    if args.iter().any(|a| a == "--selftest") {
        return run_selftest().await;
    }

    info!("Starting Fallback Shell (server mode)");

    let socket_path = common::component_socket("fallback-shell");
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    info!("Fallback Shell listening on {}", socket_path);

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_connection(stream).await;
        });
    }
}

async fn run_console() -> anyhow::Result<()> {
    let socket_path = common::bus_socket();
    let stream = tokio::net::UnixStream::connect(&socket_path).await?;
    let (mut reader, mut writer) = stream.into_split();

    println!(
        "The Machine — agent console (connected to mcp-bus at {})",
        socket_path
    );
    println!("Type '<method> <json-params>' or 'quit'. Example: state.get {{\"path\":\"/ui\"}}");

    let stdin = io::stdin();
    let mut buf = String::new();
    loop {
        print!("agent> ");
        io::stdout().flush()?;
        buf.clear();
        if stdin.read_line(&mut buf)? == 0 {
            break;
        }
        let line = buf.trim();
        if line.is_empty() {
            continue;
        }
        if line == "quit" || line == "exit" {
            break;
        }

        let (method, params) = match line.split_once(char::is_whitespace) {
            Some((m, p)) => (
                m.to_string(),
                serde_json::from_str(p.trim()).unwrap_or_else(|_| serde_json::json!({})),
            ),
            None => (line.to_string(), serde_json::json!({})),
        };

        let req = serde_json::json!({
            "id": Uuid::new_v4().to_string(),
            "method": method,
            "params": params
        });
        let mut bytes = serde_json::to_vec(&req)?;
        bytes.push(b'\n');
        writer.write_all(&bytes).await?;
        writer.flush().await?;

        let mut resp = vec![0u8; 16384];
        let n = reader.read(&mut resp).await?;
        match serde_json::from_slice::<serde_json::Value>(&resp[..n]) {
            Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()),
            Err(e) => println!("(parse error: {})", e),
        }
    }
    Ok(())
}

/// Non-interactive probe: sends a few representative requests and prints the
/// Send one MCP request over `writer`/`reader` and return the parsed response.
async fn bus_call(
    writer: &mut OwnedWriteHalf,
    reader: &mut OwnedReadHalf,
    method: &str,
    params: Value,
) -> Value {
    let req = serde_json::json!({
        "id": Uuid::new_v4().to_string(),
        "method": method,
        "params": params,
    });
    let mut bytes = serde_json::to_vec(&req).unwrap();
    bytes.push(b'\n');
    let _ = writer.write_all(&bytes).await;
    let _ = writer.flush().await;
    let mut resp = vec![0u8; 16384];
    let n = reader.read(&mut resp).await.unwrap_or(0);
    serde_json::from_slice::<Value>(&resp[..n]).unwrap_or(Value::Null)
}

/// bus responses. Used to self-verify wiring inside the initramfs.
async fn run_selftest() -> anyhow::Result<()> {
    let socket_path = common::bus_socket();
    let stream = tokio::net::UnixStream::connect(&socket_path).await?;
    let (mut reader, mut writer) = stream.into_split();

    let probes = [
        ("state.get", serde_json::json!({"path":"/ui"})),
        ("power.get_profile", serde_json::json!({})),
        ("mcp-intent:hello", serde_json::json!({})),
    ];

    for (method, params) in probes {
        let r = bus_call(&mut writer, &mut reader, method, params).await;
        println!("[selftest] {} -> {}", method, r);
    }

    // ---- Event Bus checks ----
    // 1. Publish an event that requires a decision with no handler -> AgentWake.
    let r = bus_call(
        &mut writer,
        &mut reader,
        "event.publish",
        serde_json::json!({
            "category": "health",
            "pattern": "lambda.crash",
            "payload": {"lambda": "video_player", "exit_code": 1},
            "requires_decision": true,
        }),
    )
    .await;
    let decision = r
        .get("result")
        .and_then(|v| v.get("decision"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    println!("[selftest] event.publish decision = {}", decision);

    // 2. Subscribe to external timer events.
    let r = bus_call(
        &mut writer,
        &mut reader,
        "event.subscribe",
        serde_json::json!({"category": "external", "pattern": "timer.*", "subscriber": "selftest"}),
    )
    .await;
    println!("[selftest] event.subscribe -> {}", r);

    // 3. Schedule a recurring 1s timer, then confirm it actually fires.
    let r = bus_call(
        &mut writer,
        &mut reader,
        "event.schedule",
        serde_json::json!({
            "cron": "@every 1s",
            "recurring": true,
            "category": "external",
            "pattern": "timer.fire",
            "payload": {"kind": "selftest"},
        }),
    )
    .await;
    println!("[selftest] event.schedule -> {}", r);

    let stats1 = bus_call(
        &mut writer,
        &mut reader,
        "event.stats",
        serde_json::json!({}),
    )
    .await;
    let e1 = stats1
        .get("result")
        .and_then(|v| v.get("events_emitted"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

    let stats2 = bus_call(
        &mut writer,
        &mut reader,
        "event.stats",
        serde_json::json!({}),
    )
    .await;
    let e2 = stats2
        .get("result")
        .and_then(|v| v.get("events_emitted"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let scheduled = stats2
        .get("result")
        .and_then(|v| v.get("scheduled_events"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    println!(
        "[selftest] event.stats e1={} e2={} scheduled={}",
        e1, e2, scheduled
    );

    let pass = decision == "AgentWake" && e2 > e1 && scheduled >= 1;
    println!("[selftest] EVENTBUS {}", if pass { "PASS" } else { "FAIL" });

    // ---- Lambda Server checks ----
    // 1. register a pure compute function (CAP_PURE: no OS access allowed).
    let r = bus_call(
        &mut writer,
        &mut reader,
        "lambda.register",
        serde_json::json!({
            "name": "fn-add",
            "entrypoint": "/usr/bin/fn-add",
            "capabilities": ["CAP_PURE"],
            "protocol": "line",
        }),
    )
    .await;
    println!("[selftest] lambda.register fn-add -> {}", r);

    let r = bus_call(
        &mut writer,
        &mut reader,
        "lambda.invoke",
        serde_json::json!({"name": "fn-add", "payload": {"a": 2, "b": 3}}),
    )
    .await;
    let add_stdout = r
        .get("result")
        .and_then(|v| v.get("stdout"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let add_killed = r
        .get("result")
        .and_then(|v| v.get("killed"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    println!(
        "[selftest] lambda.invoke fn-add -> stdout='{}' killed={}",
        add_stdout, add_killed
    );

    // 2. register a function that attempts a forbidden syscall (open()).
    let r = bus_call(
        &mut writer,
        &mut reader,
        "lambda.register",
        serde_json::json!({
            "name": "fn-bad",
            "entrypoint": "/usr/bin/fn-bad",
            "capabilities": ["CAP_PURE"],
        }),
    )
    .await;
    println!("[selftest] lambda.register fn-bad -> {}", r);

    let r = bus_call(
        &mut writer,
        &mut reader,
        "lambda.invoke",
        serde_json::json!({"name": "fn-bad", "payload": {}}),
    )
    .await;
    let bad_killed = r
        .get("result")
        .and_then(|v| v.get("killed"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    println!("[selftest] lambda.invoke fn-bad -> killed={}", bad_killed);

    // 3. lease fast-path.
    let r = bus_call(
        &mut writer,
        &mut reader,
        "lambda.lease",
        serde_json::json!({"name": "fn-add"}),
    )
    .await;
    let lease_id = r
        .get("result")
        .and_then(|v| v.get("lease_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    println!("[selftest] lambda.lease fn-add -> lease_id='{}'", lease_id);
    if !lease_id.is_empty() {
        let r = bus_call(
            &mut writer,
            &mut reader,
            "lambda.invoke",
            serde_json::json!({"lease_id": lease_id, "payload": {"a": 10, "b": 20}}),
        )
        .await;
        let lstdout = r
            .get("result")
            .and_then(|v| v.get("stdout"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("[selftest] lambda.invoke(lease) -> stdout='{}'", lstdout);
        let _ = bus_call(
            &mut writer,
            &mut reader,
            "lambda.stop",
            serde_json::json!({"lease_id": lease_id}),
        )
        .await;
    }

    let pass_lambda = add_stdout.contains("5") && !add_killed && bad_killed;
    println!(
        "[selftest] LAMBDA {}",
        if pass_lambda { "PASS" } else { "FAIL" }
    );

    Ok(())
}

async fn handle_connection(stream: tokio::net::UnixStream) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(msg) = serde_json::from_str::<McpMessage>(&line) {
                    let response = match msg.kind {
                        MessageKind::Request => {
                            let method = msg.method.unwrap_or_default();
                            match method.as_str() {
                                "shell.status" => {
                                    success_response(&msg.id, serde_json::json!({"active": false}))
                                }
                                "shell.activate" => {
                                    success_response(&msg.id, serde_json::json!({}))
                                }
                                "hello" => {
                                    success_response(&msg.id, serde_json::json!({"status": "ok"}))
                                }
                                _ => error_response(
                                    &msg.id,
                                    "E_NOT_FOUND",
                                    &format!("Unknown method: {}", method),
                                ),
                            }
                        }
                        _ => {
                            error_response(&msg.id, "E_INVALID_REQUEST", "Only requests supported")
                        }
                    };
                    let _ = writer
                        .write_all(serde_json::to_string(&response).unwrap().as_bytes())
                        .await;
                    let _ = writer.write_all(b"\n").await;
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }
}

fn success_response(id: &Uuid, result: serde_json::Value) -> McpMessage {
    McpMessage {
        id: *id,
        stream_id: 0,
        kind: MessageKind::Response,
        method: None,
        params: None,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: &Uuid, code: &str, message: &str) -> McpMessage {
    McpMessage {
        id: *id,
        stream_id: 0,
        kind: MessageKind::Response,
        method: None,
        params: None,
        result: None,
        error: Some(McpError {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }),
    }
}
