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
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::Barrier;
use sysinfo::{Pid, System};

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
        
        // 1. Retry Connection: Se il server è pieno, riprova per qualche secondo
        let start = Instant::now();
        let stream = loop {
            match TcpStream::connect(&addr).await {
                Ok(s) => break s,
                Err(_) => {
                    if start.elapsed().as_secs() > 10 {
                        return Err(anyhow!("Timeout TCP connect"));
                    }
                    sleep(Duration::from_millis(100)).await;
                }
            }
        };
        
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

        // 2. Register
        let register_msg = ClientMsg::Register { username: username.clone(), password: password.to_string() };
        Self::send_msg(&mut framed, &register_msg).await?;
        let _ = Self::recv_msg(&mut framed).await.and_then(|m| match m {
            ServerMsg::Registered { .. } => Ok(()),
            ServerMsg::Error { message } => Err(anyhow!("Errore registrazione: {}", message)),
            _ => Err(anyhow!("Risposta inattesa post-registrazione")),
        })?;

        // 3. Login con "Smart Retry" (Cruciale per evitare errori DB under load)
        let mut attempts = 0;
        let (token, user_id) = loop {
            let login_msg = ClientMsg::Login { username: username.clone(), password: password.to_string() };
            Self::send_msg(&mut framed, &login_msg).await?;
            
            match Self::recv_msg(&mut framed).await? {
                ServerMsg::LoginOk { session_token, user_id, .. } => break (session_token, user_id),
                ServerMsg::Error { message } => {
                    attempts += 1;
                    // Se l'errore riguarda credenziali (transazione DB non ancora committata) o lock
                    if (message.contains("Credenziali") || message.contains("locked")) && attempts < 5 {
                        sleep(Duration::from_millis(200)).await; // Aspetta e riprova
                        continue;
                    }
                    return Err(anyhow!("Login fallito definitivamente: {}", message));
                }
                _ => return Err(anyhow!("Risposta inattesa post-login")),
            }
        };

        // 4. Listen
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
    let port: u16 = rand::thread_rng().gen_range(9000..10000); 
    
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

// MONITORING OTTIMIZZATO (3s)
async fn monitor_process(pid: u32, run_flag: Arc<AtomicBool>) -> (f32, u64) {
    let mut system = System::new();
    let pid = Pid::from_u32(pid);
    let mut cpu_samples = Vec::new();
    let mut max_ram = 0;

    sleep(Duration::from_secs(2)).await;

    while run_flag.load(Ordering::Relaxed) {
        system.refresh_process(pid);
        if let Some(process) = system.process(pid) {
            cpu_samples.push(process.cpu_usage());
            let ram = process.memory();
            if ram > max_ram { max_ram = ram; }
        } else { break; }
        
        // Refresh meno frequente per salvare CPU
        sleep(Duration::from_secs(3)).await; 
    }

    let avg_cpu = if !cpu_samples.is_empty() {
        cpu_samples.iter().sum::<f32>() / cpu_samples.len() as f32
    } else { 0.0 };

    (avg_cpu, max_ram / 1024 / 1024)
}

fn get_log_path() -> std::path::PathBuf {
    let base_path = if Path::new("server/tests").exists() { "server/tests/results" } 
                   else if Path::new("tests").exists() { "tests/results" } 
                   else { "results" };
    let dir_path = Path::new(base_path);
    let _ = fs::create_dir_all(dir_path);
    dir_path.join("e2e_private_chat_monitoring_res.txt")
}

fn log_suite_header() {
    let file_path = get_log_path();
    let header = format!(
        "\n\n*********************************************\n\
        PRIVATE CHAT STRESS TEST - {} test\n\
        *********************************************\n",
        get_formatted_timestamp()
    );
    let _ = OpenOptions::new().create(true).append(true).open(&file_path).map(|mut f| f.write_all(header.as_bytes()));
}

fn log_test_result(test_num: usize, users: usize, dur: u64, int: u64, exp: u64, act: u64, cpu: f32, ram: u64) {
    let file_path = get_log_path();
    let eff = if exp > 0 { (act as f64 / exp as f64) * 100.0 } else { 0.0 };

    let entry = format!(
        "\n__________________________________________\n\
        TEST {}:\n\
        - Info: {} Users, {}s duration, {}ms interval\n\
        - Data: {}\n\
        - Performance Server:\n\
             > Avg CPU: {:.2}%\n\
             > Max RAM: {} MB\n\
        - Risultato: {} / {} messaggi (Efficienza {:.2}%)\n\
        __________________________________________\n",
        test_num, users, dur, int, get_formatted_timestamp(), cpu, ram, act, exp, eff
    );
    let _ = OpenOptions::new().create(true).append(true).open(&file_path).map(|mut f| f.write_all(entry.as_bytes()));
}

// =======================================================================================
// LOGICA SCENARIO
// =======================================================================================

async fn run_scenario(test_number: usize, n_users: usize, duration_secs: u64, interval_ms: u64) -> Result<()> {
    eprintln!("\n=== AVVIO TEST {} (Users: {}, Durata: {}s) ===", test_number, n_users, duration_secs);

    let db_path = format!("data/stress_seq_{}_{}.sqlite", test_number, unique_suffix());
    let (mut child, port) = start_server_and_wait_with_env(&[("CHAT_DB_PATH", &db_path)]).await?;
    let suffix = unique_suffix();
    
    let monitor_running = Arc::new(AtomicBool::new(true));
    let monitor_handle = {
        let pid = child.id();
        let flag = monitor_running.clone();
        tokio::spawn(async move { monitor_process(pid, flag).await })
    };

    let mut clients = Vec::with_capacity(n_users);
    for i in 0..n_users {
        let name = format!("u{}_{}_{}", test_number, i, suffix);
        
        // --- RAMP-UP CRUCIALE ---
        // 5ms di pausa evitano che 100 utenti si connettano nello stesso istante,
        // prevenendo il blocco del DB (SQLite Busy) e del network.
        if n_users >= 50 { sleep(Duration::from_millis(5)).await; }

        match TestClient::connect_and_login(port, name, "pass").await {
            Ok(c) => clients.push(c),
            Err(e) => eprintln!("Errore fatale connessione client {}: {}", i, e),
        }
    }

    if clients.is_empty() {
        let _ = child.kill();
        return Err(anyhow!("Nessun client connesso"));
    }

    // TOPOLOGIA RING
    let mut chat_ids = vec![Uuid::nil(); clients.len()];
    let usernames: Vec<String> = clients.iter().map(|c| c.username.clone()).collect();
    let n_active = clients.len();
    
    for i in 0..n_active {
        let target_idx = (i + 1) % n_active;
        let client = &mut clients[i];
        
        // Tentativo creazione chat con un minimo di tolleranza
        let _ = client.send(ClientMsg::StartPrivateChat { token: client.token, other_username: usernames[target_idx].clone() }).await;
        
        if let Ok(cid) = client.recv_until(Duration::from_secs(5), |m| {
            if let ServerMsg::PrivateChatStarted { chat_id } = m { Some(*chat_id) } else { None }
        }).await {
            chat_ids[i] = cid;
        }
    }

    // LOAD TEST
    let barrier = Arc::new(Barrier::new(n_active));
    let mut handles = Vec::new();

    for (i, mut client) in clients.into_iter().enumerate() {
        let barrier = barrier.clone();
        let chat_id = chat_ids[i];
        
        if chat_id.is_nil() { continue; }

        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let deadline = Instant::now() + Duration::from_secs(duration_secs);
            let mut count = 0;
            
            while Instant::now() < deadline {
                let content = format!("m{}", count);
                if client.send(ClientMsg::SendPrivateMessage { token: client.token, chat_id, content }).await.is_ok() {
                    // Timeout tollerante per non abortire il thread se il server lagga un attimo
                    if client.recv_until(Duration::from_secs(5), |m| match m {
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

    let mut actual_msgs = 0;
    for h in handles {
        if let Ok(Ok(c)) = h.await { actual_msgs += c; }
    }

    monitor_running.store(false, Ordering::Relaxed);
    let (avg_cpu, max_ram_mb) = monitor_handle.await.unwrap_or((0.0, 0));
    let expected_msgs = (duration_secs * 1000 / interval_ms) * n_active as u64;
    
    eprintln!("=== FINE TEST {} -> Msgs: {}/{}, CPU: {:.1}%, RAM: {}MB ===", 
        test_number, actual_msgs, expected_msgs, avg_cpu, max_ram_mb);

    log_test_result(test_number, n_active, duration_secs, interval_ms, expected_msgs, actual_msgs, avg_cpu, max_ram_mb);

    let _ = child.kill();
    let _ = std::fs::remove_file(&db_path);
    
    Ok(())
}

// =======================================================================================
// MAIN TEST RUNNER
// =======================================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn sequential_load_tests() -> Result<()> {
    log_suite_header();

    run_scenario(1, 10, 60, 1000).await?;
    run_scenario(2, 25, 60, 1000).await?;
    run_scenario(3, 50, 60, 1000).await?;
    run_scenario(4, 100, 60, 1000).await?;
    
    // Per test 5+ ricorda ulimit -n 2048 se sei su Mac
    run_scenario(5, 150, 60, 1000).await?;
    
    // Test 6 opzionale (250 utenti) - solo se hai hardware potente
    run_scenario(6, 250, 60, 1000).await?;

    Ok(())
}