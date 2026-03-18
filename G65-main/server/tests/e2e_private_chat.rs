use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use shared::{ClientMsg, ServerMsg};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout, Instant};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::bytes::Bytes;
use uuid::Uuid;
use rand::Rng;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Barrier;

// =======================================================================================
// STRUTTURE E LOGICA CLIENT
// =======================================================================================

struct TestClient {
    pub username: String,
    pub token: Uuid,
    #[allow(dead_code)]
    pub user_id: Uuid,
    pub stream: Framed<TcpStream, LengthDelimitedCodec>,
}

impl TestClient {
    pub async fn connect_and_login(port: u16, username: String, password: &str) -> Result<Self> {
        let addr = format!("127.0.0.1:{}", port);
        
        // Retry connection logic
        let stream = loop {
            match TcpStream::connect(&addr).await {
                Ok(s) => break s,
                Err(_) => sleep(Duration::from_millis(100)).await,
            }
        };
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

        // Register
        let register_msg = ClientMsg::Register { username: username.clone(), password: password.to_string() };
        Self::send_msg(&mut framed, &register_msg).await?;
        let _ = Self::recv_msg(&mut framed).await.and_then(|m| match m {
            ServerMsg::Registered { .. } => Ok(()),
            ServerMsg::Error { message } => Err(anyhow!("Errore registrazione: {}", message)),
            _ => Err(anyhow!("Risposta inattesa post-registrazione")),
        })?;

        // Login
        let login_msg = ClientMsg::Login { username: username.clone(), password: password.to_string() };
        Self::send_msg(&mut framed, &login_msg).await?;
        let (token, user_id) = Self::recv_msg(&mut framed).await.and_then(|m| match m {
            ServerMsg::LoginOk { session_token, user_id, .. } => Ok((session_token, user_id)),
            ServerMsg::Error { message } => Err(anyhow!("Errore login: {}", message)),
            _ => Err(anyhow!("Risposta inattesa post-login")),
        })?;

        // Listen
        let listen_msg = ClientMsg::Listen { token };
        Self::send_msg(&mut framed, &listen_msg).await?;

        Ok(Self { username, token, user_id, stream: framed })
    }

    async fn send_msg(framed: &mut Framed<TcpStream, LengthDelimitedCodec>, msg: &ClientMsg) -> Result<()> {
        let bytes = serde_json::to_vec(msg)?;
        framed.send(Bytes::from(bytes)).await?;
        Ok(())
    }

    async fn recv_msg(framed: &mut Framed<TcpStream, LengthDelimitedCodec>) -> Result<ServerMsg> {
        let packet = timeout(Duration::from_secs(10), framed.next()).await
            .map_err(|_| anyhow!("Timeout ricezione msg"))?
            .ok_or(anyhow!("Stream chiuso"))??;
        let msg: ServerMsg = serde_json::from_slice(&packet)?;
        Ok(msg)
    }

    pub async fn send(&mut self, msg: ClientMsg) -> Result<()> {
        Self::send_msg(&mut self.stream, &msg).await
    }

    pub async fn recv_until<T>(&mut self, dur: Duration, mut pred: impl FnMut(&ServerMsg) -> Option<T>) -> Result<T> {
        let deadline = Instant::now() + dur;
        loop {
            let remain = deadline.saturating_duration_since(Instant::now());
            if remain.is_zero() { return Err(anyhow!("Timeout recv_until scaduto")); }
            
            match timeout(remain, self.stream.next()).await {
                Ok(Some(Ok(bytes))) => {
                    if let Ok(msg) = serde_json::from_slice::<ServerMsg>(&bytes) {
                        if let Some(res) = pred(&msg) { return Ok(res); }
                        if let ServerMsg::Error { message } = &msg {
                            eprintln!("[SERVER ERROR to {}]: {}", self.username, message);
                        }
                    }
                }
                Ok(None) => return Err(anyhow!("Connessione chiusa dal server")),
                Ok(Some(Err(e))) => return Err(anyhow!("Errore codec: {}", e)),
                Err(_) => return Err(anyhow!("Timeout scaduto")),
            }
        }
    }
}

// =======================================================================================
// HELPER FUNCTIONS
// =======================================================================================

async fn start_server_and_wait_with_env(envs: &[(&str, &str)]) -> Result<(std::process::Child, u16)> {
    let _ = std::fs::create_dir_all("data");
    let server_exe = env!("CARGO_BIN_EXE_server");
    let port: u16 = rand::thread_rng().gen_range(9000..10000); // Usa porte random per evitare conflitti
    
    let mut cmd = Command::new(server_exe);
    cmd.env("PORT", &port.to_string());
    for (k, v) in envs { cmd.env(k, v); }

    let mut child = cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit()).spawn().expect("Impossibile avviare il server.");
    let addr = format!("127.0.0.1:{}", port);
    for _ in 0..50 {
        if TcpStream::connect(&addr).await.is_ok() { return Ok((child, port)); }
        sleep(Duration::from_millis(100)).await;
    }
    Ok((child, port))
}

fn unique_suffix() -> String {
    let mut rng = rand::thread_rng();
    format!("{:x}", rng.r#gen::<u32>())
}

fn get_formatted_timestamp() -> String {
    let output = Command::new("date").arg("+%d/%m/%Y %H:%M").output().ok();
    if let Some(o) = output {
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    } else {
        "Data Sconosciuta".to_string()
    }
}

/// Helper per ottenere il percorso del file di log
fn get_log_path() -> std::path::PathBuf {
    let base_path = if Path::new("server/tests").exists() { "server/tests/results" } 
                   else if Path::new("tests").exists() { "tests/results" } 
                   else { "results" };
    
    let dir_path = Path::new(base_path);
    if let Err(_) = fs::create_dir_all(dir_path) {
        eprintln!("Errore creazione directory log");
    }
    dir_path.join("e2e_private_chat_res.txt")
}

/// SCRIVE L'INTESTAZIONE PRINCIPALE DEL TEST SUITE (Nuova Funzione)
fn log_suite_header() {
    let file_path = get_log_path();
    let timestamp = get_formatted_timestamp();

    let header = format!(
        "\n\n*********************************************\n\
        PRIVATE CHAT STRESS TEST - {} test\n\
        *********************************************\n",
        timestamp
    );

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&file_path) {
        let _ = file.write_all(header.as_bytes());
    }
}

/// Scrive il log formattato come richiesto
fn log_test_result(
    test_number: usize,
    n_users: usize,
    duration: u64,
    interval: u64,
    expected: u64,
    actual: u64
) {
    let file_path = get_log_path();
    let timestamp = get_formatted_timestamp();
    
    // Calcolo efficienza
    let efficiency = if expected > 0 {
        (actual as f64 / expected as f64) * 100.0
    } else { 0.0 };

    let log_entry = format!(
        "\n__________________________________________\n\
        TEST {}:\n\
        - Info: {} Users, {} sec duration, {} ms interval\n\
        - Data: {}\n\
        - Risultato: {} / {} messaggi (Efficienza {:.2}%)\n\
        __________________________________________\n",
        test_number, n_users, duration, interval, timestamp, actual, expected, efficiency
    );

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&file_path) {
        let _ = file.write_all(log_entry.as_bytes());
    }
}

// =======================================================================================
// LOGICA SCENARIO SINGOLO
// =======================================================================================

/// Esegue un singolo scenario di test, avviando un proprio server e chiudendolo alla fine
async fn run_scenario(test_number: usize, n_users: usize, duration_secs: u64, interval_ms: u64) -> Result<()> {
    eprintln!("\n=== AVVIO TEST {} (Users: {}, Durata: {}s) ===", test_number, n_users, duration_secs);

    // 1. Avvio Server Pulito
    let db_path = format!("data/stress_seq_{}_{}.sqlite", test_number, unique_suffix());
    let (mut child, port) = start_server_and_wait_with_env(&[("CHAT_DB_PATH", &db_path)]).await?;
    let suffix = unique_suffix();
    
    // 2. Creazione Client
    let mut clients = Vec::with_capacity(n_users);
    for i in 0..n_users {
        let name = format!("u{}_{}_{}", test_number, i, suffix);
        // Ramp-up: Pausa minima per evitare il blocco del server a 100+ utenti
        if n_users >= 50 { sleep(Duration::from_millis(5)).await; }
        
        let c = TestClient::connect_and_login(port, name, "pass").await?;
        clients.push(c);
    }

    // 3. Creazione Anello Chat
    let mut chat_ids = vec![Uuid::nil(); n_users];
    let usernames: Vec<String> = clients.iter().map(|c| c.username.clone()).collect();
    
    for i in 0..n_users {
        let target_idx = (i + 1) % n_users;
        let client = &mut clients[i];
        client.send(ClientMsg::StartPrivateChat { token: client.token, other_username: usernames[target_idx].clone() }).await?;
        
        let cid = client.recv_until(Duration::from_secs(5), |m| {
            if let ServerMsg::PrivateChatStarted { chat_id } = m { Some(*chat_id) } else { None }
        }).await?;
        chat_ids[i] = cid;
    }

    // 4. Load Test
    let barrier = Arc::new(Barrier::new(n_users));
    let mut handles = Vec::new();

    for (i, mut client) in clients.into_iter().enumerate() {
        let barrier = barrier.clone();
        let chat_id = chat_ids[i];
        
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let deadline = Instant::now() + Duration::from_secs(duration_secs);
            let mut count = 0;
            
            while Instant::now() < deadline {
                let content = format!("m{}", count);
                if client.send(ClientMsg::SendPrivateMessage { token: client.token, chat_id, content }).await.is_ok() {
                    if client.recv_until(Duration::from_secs(2), |m| match m {
                        ServerMsg::PrivateMessageSent { .. } => Some(()),
                        _ => None
                    }).await.is_ok() {
                        count += 1;
                    }
                }
                sleep(Duration::from_millis(interval_ms)).await;
            }
            Ok::<u64, anyhow::Error>(count)
        }));
    }

    // 5. Raccolta Dati
    let mut actual_msgs = 0;
    for h in handles {
        if let Ok(Ok(c)) = h.await { actual_msgs += c; }
    }

    let expected_msgs = (duration_secs * 1000 / interval_ms) * n_users as u64;
    eprintln!("=== FINE TEST {} -> Processati: {} / Attesi: {} ===", test_number, actual_msgs, expected_msgs);

    // 6. Logging
    log_test_result(test_number, n_users, duration_secs, interval_ms, expected_msgs, actual_msgs);

    // 7. Cleanup
    let _ = child.kill();
    let _ = std::fs::remove_file(&db_path);
    
    Ok(())
}

// =======================================================================================
// MAIN TEST RUNNER
// =======================================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 16)] // Aumentato thread count per gestire 150 user
async fn sequential_load_tests() -> Result<()> {
    // *** SCRITTURA HEADER LOG ***
    log_suite_header();

    // TEST 1: 10 user
    run_scenario(1, 10, 60, 1000).await?;

    // TEST 2: 25 user
    run_scenario(2, 25, 60, 1000).await?;

    // TEST 3: 50 user
    run_scenario(3, 50, 60, 1000).await?;

    // TEST 4: 100 user
    run_scenario(4, 100, 60, 1000).await?;

    // TEST 5: 150 user
    run_scenario(5, 150, 60, 1000).await?;

    // TEST 6: 250 user
    run_scenario(6, 250, 60, 1000).await?;

    Ok(())
}