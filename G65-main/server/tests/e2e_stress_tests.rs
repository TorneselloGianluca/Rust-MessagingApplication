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
use std::path::Path; // Aggiunto per gestire i percorsi in modo intelligente

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
        let stream = TcpStream::connect(&addr).await?;
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

        // 1. Register
        let register_msg = ClientMsg::Register {
            username: username.clone(),
            password: password.to_string()
        };
        Self::send_msg(&mut framed, &register_msg).await?;

        // Attendi risposta Registered
        let _ = Self::recv_msg(&mut framed).await.and_then(|m| match m {
            ServerMsg::Registered { .. } => Ok(()),
            ServerMsg::Error { message } => Err(anyhow!("Errore registrazione: {}", message)),
            _ => Err(anyhow!("Risposta inattesa post-registrazione")),
        })?;

        // 2. Login
        let login_msg = ClientMsg::Login {
            username: username.clone(),
            password: password.to_string()
        };
        Self::send_msg(&mut framed, &login_msg).await?;

        // Attendi risposta LoginOk
        let (token, user_id) = Self::recv_msg(&mut framed).await.and_then(|m| match m {
            ServerMsg::LoginOk { session_token, user_id, .. } => Ok((session_token, user_id)),
            ServerMsg::Error { message } => Err(anyhow!("Errore login: {}", message)),
            _ => Err(anyhow!("Risposta inattesa post-login")),
        })?;

        // 3. Listen
        let listen_msg = ClientMsg::Listen { token };
        Self::send_msg(&mut framed, &listen_msg).await?;

        Ok(Self {
            username,
            token,
            user_id,
            stream: framed,
        })
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

    pub async fn recv_until<T>(
        &mut self,
        dur: Duration,
        mut pred: impl FnMut(&ServerMsg) -> Option<T>,
    ) -> Result<T> {
        let deadline = Instant::now() + dur;
        loop {
            let remain = deadline.saturating_duration_since(Instant::now());
            if remain.is_zero() {
                return Err(anyhow!("Timeout recv_until scaduto"));
            }

            let packet_opt = timeout(remain, self.stream.next()).await;

            match packet_opt {
                Ok(Some(Ok(bytes))) => {
                    if let Ok(msg) = serde_json::from_slice::<ServerMsg>(&bytes) {
                        if let Some(res) = pred(&msg) {
                            return Ok(res);
                        }
                        if let ServerMsg::Error { message } = msg {
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
// HELPER FUNCTIONS (Server & Logging)
// =======================================================================================

async fn start_server_and_wait_with_env(envs: &[(&str, &str)]) -> Result<(std::process::Child, u16)> {
    let _ = std::fs::create_dir_all("data");
    let server_exe = env!("CARGO_BIN_EXE_server");

    let port: u16 = rand::thread_rng().gen_range(8000..9000);
    let port_str = port.to_string();

    let mut cmd = Command::new(server_exe);
    cmd.env("PORT", &port_str);
    for (k, v) in envs {
        cmd.env(k, v);
    }

    let mut child = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Impossibile avviare il server. Fai `cargo build --bin server` prima.");

    let addr = format!("127.0.0.1:{}", port);
    for _ in 0..50 {
        if TcpStream::connect(&addr).await.is_ok() {
            return Ok((child, port));
        }
        sleep(Duration::from_millis(100)).await;
    }
    let _ = child.kill();
    Err(anyhow!("Il server non è partito su {}", addr))
}

fn unique_suffix() -> String {
    let mut rng = rand::thread_rng();
    format!("{:x}", rng.r#gen::<u32>())
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

/// Funzione per scrivere i risultati nel file di log
/// Rileva dinamicamente se siamo in ROOT o in SERVER per posizionare il file correttamente in:
/// .../server/tests/results/e2e_stress_tests_res.txt
fn log_test_result(scenario_label: &str, test_type: &str, details: &str, result_summary: &str) {
    
    // Logica dinamica per capire il percorso base
    let base_path = if Path::new("server/tests").exists() {
        // Siamo nella root (G65)
        "server/tests/results"
    } else if Path::new("tests").exists() {
        // Siamo dentro server (G65/server)
        "tests/results"
    } else {
        // Fallback: siamo già dentro tests? (Raro ma possibile)
        "results"
    };

    let dir_path = Path::new(base_path);
    let file_path = dir_path.join("e2e_stress_tests_res.txt");

    // Crea directory se non esiste
    if let Err(e) = fs::create_dir_all(dir_path) {
        eprintln!("[LOG ERROR] Impossibile creare directory {:?}: {}", dir_path, e);
        return; 
    }

    let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => format!("{}", d.as_secs()),
        Err(_) => "unknown".to_string(),
    };

    let log_entry = format!(
        "\n########################\n\
        STRESS TEST - timestamp: {}\n\
        __________________________________\n\
        {}:\n\
        - Tipo: {}\n\
        - Dati: {}\n\
        - Risultati: {}\n\
        ##########################\n",
        timestamp, scenario_label, test_type, details, result_summary
    );

    match OpenOptions::new().create(true).append(true).open(&file_path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                eprintln!("[LOG ERROR] Scrittura fallita: {}", e);
            }
        },
        Err(e) => eprintln!("[LOG ERROR] Apertura file fallita: {}", e),
    }
}

// =======================================================================================
// TESTS
// =======================================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn e2e_group_debug() -> Result<()> {
    let db_path = format!("data/stress_debug_{}.sqlite", unique_suffix());
    let (mut child, port) = start_server_and_wait_with_env(&[("CHAT_DB_PATH", &db_path)]).await?;

    let suffix = unique_suffix();
    let alice_name = format!("alice_{}", suffix);
    let bob_name = format!("bob_{}", suffix);

    let mut alice = TestClient::connect_and_login(port, alice_name.clone(), "pass").await?;
    let mut bob = TestClient::connect_and_login(port, bob_name.clone(), "pass").await?;

    let alice_token = alice.token;
    alice.send(ClientMsg::CreateGroup {
        token: alice_token,
        name: "TestGroup".to_string()
    }).await?;

    let group_id = alice.recv_until(Duration::from_secs(5), |m| {
        if let ServerMsg::GroupCreated { group_id } = m { Some(*group_id) } else { None }
    }).await?;
    eprintln!("[SETUP] Gruppo creato: {}", group_id);

    alice.send(ClientMsg::AddGroupMember {
        token: alice_token,
        group_id,
        username: bob_name.clone()
    }).await?;

    alice.recv_until(Duration::from_secs(5), |m| match m {
        ServerMsg::GroupMemberAdded => Some(()),
        _ => None
    }).await?;
    eprintln!("[SETUP] Bob aggiunto al gruppo");

    let rate_ms = env_u64("LOAD_MS", 50);
    let duration_secs = env_u64("LOAD_DURATION_SECS", 5);
    let deadline = Instant::now() + Duration::from_secs(duration_secs);
    let mut ticker = tokio::time::interval(Duration::from_millis(rate_ms));

    let mut counter = 0;
    while Instant::now() < deadline {
        ticker.tick().await;
        let (sender, _other) = if counter % 2 == 0 { (&mut alice, &mut bob) } else { (&mut bob, &mut alice) };
        let content = format!("msg #{}", counter);
        let token = sender.token;

        sender.send(ClientMsg::SendGroupMessage { token, group_id, content }).await?;
        sender.recv_until(Duration::from_secs(1), |m| match m {
            ServerMsg::GroupMessageSent { .. } => Some(()),
            _ => None
        }).await?;
        counter += 1;
    }

    eprintln!("[DONE] Inviati {} messaggi", counter);

    // LOGGING
    log_test_result(
        "Scenario A",
        "Group Debug (Ping-Pong)",
        &format!("Durata: {}s, Rate: {}ms", duration_secs, rate_ms),
        &format!("OK - Scambiati {} messaggi", counter)
    );

    let _ = child.kill();
    let _ = std::fs::remove_file(&db_path);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn e2e_true_async_burst() -> Result<()> {
    let db_path = format!("data/stress_true_burst_{}.sqlite", unique_suffix());
    let (mut child, port) = start_server_and_wait_with_env(&[("CHAT_DB_PATH", &db_path)]).await?;

    let suffix = unique_suffix();
    let mut alice = TestClient::connect_and_login(port, format!("alice_tb_{}", suffix), "pass").await?;
    let token = alice.token;

    // Crea gruppo
    alice.send(ClientMsg::CreateGroup { token, name: "BurstGroup".into() }).await?;
    let gid = alice.recv_until(Duration::from_secs(5), |m| {
        if let ServerMsg::GroupCreated { group_id } = m { Some(*group_id) } else { None }
    }).await?;

    let burst_size = 100; // Invia 100 messaggi "a raffica"
    eprintln!("[TRUE BURST] Invio {} messaggi senza attendere ACK...", burst_size);

    // 1. INVIA TUTTO INSIEME (Flood)
    for i in 0..burst_size {
        alice.send(ClientMsg::SendGroupMessage {
            token,
            group_id: gid,
            content: format!("burst msg {}", i),
        }).await?;
    }

    // 2. ORA RACCOGLI GLI ACK
    eprintln!("[TRUE BURST] Attesa ACK...");
    let start = Instant::now();
    let mut acks = 0;

    let receive_loop = async {
        loop {
            if acks >= burst_size { return Ok::<(), anyhow::Error>(()); }
            alice.recv_until(Duration::from_secs(5), |m| match m {
                ServerMsg::GroupMessageSent { .. } => {
                    acks += 1;
                    if acks % 20 == 0 { eprintln!("   ...ricevuti {}/{}", acks, burst_size); }
                    Some(())
                },
                _ => None
            }).await?;
        }
    };

    timeout(Duration::from_secs(10), receive_loop).await
        .map_err(|_| anyhow!("Timeout! Ricevuti solo {}/{} ACK", acks, burst_size))??;

    let elapsed = start.elapsed();
    eprintln!("[TRUE BURST] Successo! {} msg processati in {:.2?}", burst_size, elapsed);

    // LOGGING
    log_test_result(
        "Scenario B",
        "True Async Burst (Flooding)",
        &format!("Burst size: {} msgs", burst_size),
        &format!("OK - Processati in {:.2?}", elapsed)
    );

    let _ = child.kill();
    let _ = std::fs::remove_file(&db_path);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn e2e_read_write_pressure() -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::Barrier;

    let db_path = format!("data/stress_rw_{}.sqlite", unique_suffix());
    let (mut child, port) = start_server_and_wait_with_env(&[("CHAT_DB_PATH", &db_path)]).await?;

    // Setup: 1 Scrittore (Alice), 5 Lettori (Readers)
    let suffix = unique_suffix();
    let mut writer = TestClient::connect_and_login(port, format!("writer_{}", suffix), "p").await?;

    // Creazione Gruppo e popolamento iniziale
    writer.send(ClientMsg::CreateGroup { token: writer.token, name: "RW_Group".into() }).await?;
    let gid = writer.recv_until(Duration::from_secs(5), |m| {
        if let ServerMsg::GroupCreated { group_id } = m { Some(*group_id) } else { None }
    }).await?;

    // Popola con messaggi iniziali
    for i in 0..50 {
        writer.send(ClientMsg::SendGroupMessage { token: writer.token, group_id: gid, content: format!("init {}", i) }).await?;
        writer.recv_until(Duration::from_millis(500), |m| match m { ServerMsg::GroupMessageSent{..} => Some(()), _ => None }).await?;
    }

    let n_readers = 5;
    let mut readers = Vec::new();
    for i in 0..n_readers {
        let r = TestClient::connect_and_login(port, format!("reader_{}_{}", i, suffix), "p").await?;
        writer.send(ClientMsg::AddGroupMember { token: writer.token, group_id: gid, username: r.username.clone() }).await?;
        writer.recv_until(Duration::from_secs(2), |m| match m { ServerMsg::GroupMemberAdded => Some(()), _ => None }).await?;
        readers.push(r);
    }

    let barrier = Arc::new(Barrier::new(n_readers + 1));
    let mut handles = Vec::new();

    // --- AVVIO READER TASKS ---
    for mut r in readers {
        let b = barrier.clone();
        let my_token = r.token;
        handles.push(tokio::spawn(async move {
            b.wait().await;
            for _ in 0..20 {
                r.send(ClientMsg::GetGroupMessages { token: my_token, group_id: gid, limit: 50 }).await?;
                r.recv_until(Duration::from_secs(2), |m| match m {
                    ServerMsg::GroupMessages { .. } => Some(()),
                    _ => None
                }).await?;
            }
            Ok::<(), anyhow::Error>(())
        }));
    }

    // --- WRITER TASK ---
    let b = barrier.clone();
    let w_handle = tokio::spawn(async move {
        b.wait().await;
        for i in 0..20 {
            writer.send(ClientMsg::SendGroupMessage {
                token: writer.token,
                group_id: gid,
                content: format!("contention msg {}", i)
            }).await?;

            writer.recv_until(Duration::from_secs(5), |m| match m {
                ServerMsg::GroupMessageSent { .. } => Some(()),
                _ => None
            }).await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    handles.push(w_handle);

    for h in handles {
        h.await??;
    }

    eprintln!("[RW PRESSURE] Sopravvissuto a letture e scritture concorrenti.");

    // LOGGING
    log_test_result(
        "Scenario C",
        "Read/Write Pressure",
        &format!("1 Writer vs {} Readers", n_readers),
        "OK - Nessun deadlock o errore DB"
    );

    let _ = child.kill();
    let _ = std::fs::remove_file(&db_path);
    Ok(())
}