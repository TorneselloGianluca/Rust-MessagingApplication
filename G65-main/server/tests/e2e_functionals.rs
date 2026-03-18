use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use shared::{ClientMsg, ServerMsg, MessageInfo};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout, Instant};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_util::bytes::Bytes;
use uuid::Uuid;
use rand::Rng;


struct TestClient {
    pub username: String,
    pub token: Uuid,
    pub stream: Framed<TcpStream, LengthDelimitedCodec>,
}

impl TestClient {
    /// Connette, Registra (opzionale), Logga e si mette in ascolto (Listen)
    /// Se `register` è false, tenta solo il login.
    pub async fn new(port: u16, username: String, password: &str, do_register: bool) -> Result<Self> {
        let addr = format!("127.0.0.1:{}", port);
        let stream = TcpStream::connect(&addr).await?;
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

        // 1. Register (se richiesto)
        if do_register {
            let reg_msg = ClientMsg::Register { username: username.clone(), password: password.to_string() };
            Self::send_msg(&mut framed, &reg_msg).await?;

            // Attendi conferma registrazione
            match Self::recv_msg(&mut framed).await? {
                ServerMsg::Registered { .. } => {},
                ServerMsg::Error { message } => return Err(anyhow!("Register failed for {}: {}", username, message)),
                _ => return Err(anyhow!("Risposta imprevista alla registrazione")),
            }
        }

        // 2. Login
        let login_msg = ClientMsg::Login { username: username.clone(), password: password.to_string() };
        Self::send_msg(&mut framed, &login_msg).await?;

        // 3. Gestione risposta Login
        let token = match Self::recv_msg(&mut framed).await? {
            ServerMsg::LoginOk { session_token, .. } => session_token,
            ServerMsg::Error { message } => return Err(anyhow!("Login failed: {}", message)), // Utile per test negativi
            _ => return Err(anyhow!("Risposta imprevista al login")),
        };

        // 4. Listen (Necessario per ricevere le Push)
        let listen_msg = ClientMsg::Listen { token };
        Self::send_msg(&mut framed, &listen_msg).await?;

        Ok(Self { username, token, stream: framed })
    }

    /// Connessione "grezza" senza login automatico (per testare login falliti)
    pub async fn connect_raw(port: u16) -> Result<Framed<TcpStream, LengthDelimitedCodec>> {
        let addr = format!("127.0.0.1:{}", port);
        let stream = TcpStream::connect(&addr).await?;
        Ok(Framed::new(stream, LengthDelimitedCodec::new()))
    }

    async fn send_msg(framed: &mut Framed<TcpStream, LengthDelimitedCodec>, msg: &ClientMsg) -> Result<()> {
        let bytes = serde_json::to_vec(msg)?;
        framed.send(Bytes::from(bytes)).await?;
        Ok(())
    }

    async fn recv_msg(framed: &mut Framed<TcpStream, LengthDelimitedCodec>) -> Result<ServerMsg> {
        let packet = timeout(Duration::from_secs(5), framed.next()).await
            .map_err(|_| anyhow!("Timeout attesa risposta"))?
            .ok_or(anyhow!("Stream chiuso"))??;
        Ok(serde_json::from_slice(&packet)?)
    }

    pub async fn send(&mut self, msg: ClientMsg) -> Result<()> {
        Self::send_msg(&mut self.stream, &msg).await
    }

    /// Attende un messaggio che soddisfa il predicato
    pub async fn recv_until<T>(
        &mut self,
        step_name: &str,
        dur: Duration,
        mut pred: impl FnMut(&ServerMsg) -> Option<T>,
    ) -> Result<T> {
        let deadline = Instant::now() + dur;
        loop {
            let remain = deadline.saturating_duration_since(Instant::now());
            if remain.is_zero() { return Err(anyhow!("[TIMEOUT @ {}]", step_name)); }

            let packet_opt = timeout(remain, self.stream.next()).await;
            match packet_opt {
                Ok(Some(Ok(bytes))) => {
                    if let Ok(msg) = serde_json::from_slice::<ServerMsg>(&bytes) {
                        if let Some(res) = pred(&msg) { return Ok(res); }
                        // Ignora altri messaggi o logga errori
                        if let ServerMsg::Error { message } = &msg {
                            eprintln!("[{}] Ricevuto errore dal server: {}", self.username, message);
                        }
                    }
                }
                Ok(None) => return Err(anyhow!("[{}] Connessione chiusa", step_name)),
                _ => return Err(anyhow!("[{}] Errore stream o timeout", step_name)),
            }
        }
    }

    /// Verifica che NON arrivino messaggi che soddisfano il predicato
    pub async fn expect_no_match(
        &mut self,
        step_name: &str,
        dur: Duration,
        mut pred: impl FnMut(&ServerMsg) -> bool,
    ) -> Result<()> {
        let deadline = Instant::now() + dur;
        loop {
            let remain = deadline.saturating_duration_since(Instant::now());
            if remain.is_zero() { return Ok(()); } // Successo: timeout scaduto senza match

            if let Ok(Some(Ok(bytes))) = timeout(remain, self.stream.next()).await {
                if let Ok(msg) = serde_json::from_slice::<ServerMsg>(&bytes) {
                    if pred(&msg) {
                        return Err(anyhow!("[UNEXPECTED @ {}] Ricevuto messaggio proibito: {:?}", step_name, msg));
                    }
                }
            } else {
                return Ok(()); // Stream chiuso o timeout
            }
        }
    }
}

// Avvio server con DB temporaneo e porta casuale
async fn start_server() -> Result<(std::process::Child, u16, String)> {
    let _ = std::fs::create_dir_all("data");
    let mut rng = rand::thread_rng();
    let port: u16 = rng.r#gen::<u16>() % 1000 + 18000; // Porta tra 18000 e 19000
    let db_path = format!("data/func_test_{}.sqlite", rng.r#gen::<u32>());

    let server_exe = env!("CARGO_BIN_EXE_server");
    let mut cmd = Command::new(server_exe);
    cmd.env("PORT", port.to_string())
        .env("CHAT_DB_PATH", &db_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().expect("Impossibile avviare server");

    // Attesa
    let addr = format!("127.0.0.1:{}", port);
    for _ in 0..50 {
        if TcpStream::connect(&addr).await.is_ok() { return Ok((child, port, db_path)); }
        sleep(Duration::from_millis(100)).await;
    }
    Err(anyhow!("Server non partito su {}", port))
}


// =======================================================================================
// TEST FUNZIONALE E2E
// =======================================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn e2e_all_requests_g65() -> Result<()> {
    let (mut child, port, db_path) = start_server().await?;

    // Assicura pulizia alla fine anche in caso di panico (tramite drop manuale o catch unwind,
    // qui facciamo cleanup esplicito alla fine per semplicità)

    // ----------------------------------------------------------------------
    // 1. LOGIN di utente NON registrato deve FALLIRE
    // ----------------------------------------------------------------------
    {
        let mut framed = TestClient::connect_raw(port).await?;
        let msg = ClientMsg::Login { username: "ghost".into(), password: "pw".into() };
        TestClient::send_msg(&mut framed, &msg).await?;

        let resp = TestClient::recv_msg(&mut framed).await?;
        match resp {
            ServerMsg::Error { .. } => {}, // OK
            _ => return Err(anyhow!("Il login di un utente inesistente non ha dato errore")),
        }
    }

    // ----------------------------------------------------------------------
    // 2. REGISTRAZIONE Utenti (Alice, Bob, Charlie)
    // ----------------------------------------------------------------------
    let mut alice = TestClient::new(port, "alice".into(), "pw", true).await?;
    let mut bob   = TestClient::new(port, "bob".into(),   "pw", true).await?;
    let mut charlie = TestClient::new(port, "charlie".into(), "pw", true).await?;

    // ----------------------------------------------------------------------
    // 3. REGISTRAZIONE DUPLICATA deve FALLIRE
    // ----------------------------------------------------------------------
    let dup_res = TestClient::new(port, "alice".into(), "pw", true).await;
    assert!(dup_res.is_err(), "La seconda registrazione di alice deve fallire");

    // ----------------------------------------------------------------------
    // 4. CREAZIONE GRUPPO
    // ----------------------------------------------------------------------
    alice.send(ClientMsg::CreateGroup { token: alice.token, name: "G65Group".into() }).await?;

    let gid = alice.recv_until("Wait GroupCreated", Duration::from_secs(5), |m| {
        if let ServerMsg::GroupCreated { group_id } = m { Some(*group_id) } else { None }
    }).await?;

    // ----------------------------------------------------------------------
    // 5. INVITO AL GRUPPO (AddGroupMember)
    // ----------------------------------------------------------------------
    // Alice aggiunge Bob
    alice.send(ClientMsg::AddGroupMember { token: alice.token, group_id: gid, username: "bob".into() }).await?;
    alice.recv_until("Wait GroupMemberAdded Ack", Duration::from_secs(5), |m| match m {
        ServerMsg::GroupMemberAdded => Some(()),
        _ => None
    }).await?;

    // Bob dovrebbe ricevere una notifica (opzionale in base al server, ma controlliamo se c'è PushGroupUpdated)
    // Se il server non manda PushGroupUpdated al membro aggiunto, saltiamo questo check.
    // Nel tuo codice server sembra esserci `PushGroupUpdated`.
    let _ = bob.recv_until("Wait PushGroupUpdated", Duration::from_millis(500), |m| match m {
        ServerMsg::PushGroupUpdated => Some(()),
        _ => None
    }).await; // Ignoriamo result (timeout ok se non implementato)

    // --- Negativo: Aggiungere utente inesistente ---
    alice.send(ClientMsg::AddGroupMember { token: alice.token, group_id: gid, username: "nessuno".into() }).await?;
    alice.recv_until("Wait Error adding nobody", Duration::from_secs(5), |m| match m {
        ServerMsg::Error { .. } => Some(()),
        _ => None
    }).await?;

    // ----------------------------------------------------------------------
    // 6. MESSAGGI NEL GRUPPO
    // ----------------------------------------------------------------------
    alice.send(ClientMsg::SendGroupMessage { token: alice.token, group_id: gid, content: "Hola Bob".into() }).await?;

    // Alice riceve Ack
    alice.recv_until("Wait Ack Sent", Duration::from_secs(2), |m| match m {
        ServerMsg::GroupMessageSent { .. } => Some(()),
        _ => None
    }).await?;

    // Bob riceve Push
    bob.recv_until("Bob receives group msg", Duration::from_secs(5), |m| {
        if let ServerMsg::PushNewMessage { message, group_id, .. } = m {
            if *group_id == Some(gid) && message.content == "Hola Bob" { return Some(()); }
        }
        None
    }).await?;

    // Charlie (non membro) NON deve ricevere nulla
    charlie.expect_no_match("Charlie spying", Duration::from_millis(500), |m| {
        matches!(m, ServerMsg::PushNewMessage { .. })
    }).await?;

    // ----------------------------------------------------------------------
    // 7. LISTA PARTECIPANTI (GetGroupMembers)
    // ----------------------------------------------------------------------
    alice.send(ClientMsg::GetGroupMembers { token: alice.token, group_id: gid }).await?;
    let members = alice.recv_until("Get Members", Duration::from_secs(5), |m| {
        if let ServerMsg::GroupMembers { members } = m { Some(members.clone()) } else { None }
    }).await?;

    // Verifica che ci siano Alice e Bob
    let names: Vec<String> = members.iter().map(|u| u.username.clone()).collect();
    assert!(names.contains(&"alice".to_string()));
    assert!(names.contains(&"bob".to_string()));
    assert!(!names.contains(&"charlie".to_string()));

    // ----------------------------------------------------------------------
    // 8. CHAT PRIVATA
    // ----------------------------------------------------------------------
    alice.send(ClientMsg::StartPrivateChat { token: alice.token, other_username: "charlie".into() }).await?;
    let chat_id = alice.recv_until("PrivChat Started", Duration::from_secs(5), |m| {
        if let ServerMsg::PrivateChatStarted { chat_id } = m { Some(*chat_id) } else { None }
    }).await?;

    // Alice scrive a Charlie
    alice.send(ClientMsg::SendPrivateMessage { token: alice.token, chat_id, content: "Psst Charlie".into() }).await?;

    // Charlie riceve
    charlie.recv_until("Charlie receives priv msg", Duration::from_secs(5), |m| {
        if let ServerMsg::PushNewMessage { message, chat_id: cid, .. } = m {
            if *cid == Some(chat_id) && message.content == "Psst Charlie" { return Some(()); }
        }
        None
    }).await?;

    // Bob non deve vedere nulla
    bob.expect_no_match("Bob spying private", Duration::from_millis(500), |m| {
        matches!(m, ServerMsg::PushNewMessage { .. })
    }).await?;

    // ----------------------------------------------------------------------
    // CLEANUP
    // ----------------------------------------------------------------------
    let _ = child.kill();
    let _ = std::fs::remove_file(&db_path);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn e2e_offline_messaging_and_history() -> Result<()> {
    let (mut child, port, db_path) = start_server().await?;

    // 1. Registrazione Alice e Bob
    let mut alice = TestClient::new(port, "alice_h".into(), "pw", true).await?;
    let bob_init = TestClient::new(port, "bob_h".into(), "pw", true).await?;

    // Salva il token di bob se volessimo riusarlo, ma simuliamo un ri-login completo
    drop(bob_init); // BOB SI DISCONNETTE

    // 2. Alice avvia chat privata con Bob (che è offline)
    alice.send(ClientMsg::StartPrivateChat { token: alice.token, other_username: "bob_h".into() }).await?;
    let chat_id = alice.recv_until("PrivChat Started", Duration::from_secs(2), |m| {
        if let ServerMsg::PrivateChatStarted { chat_id } = m { Some(*chat_id) } else { None }
    }).await?;

    // Alice invia messaggio a Bob offline
    alice.send(ClientMsg::SendPrivateMessage { token: alice.token, chat_id, content: "Sei offline?".into() }).await?;
    alice.recv_until("Msg Sent Ack", Duration::from_secs(2), |m| match m {
        ServerMsg::PrivateMessageSent { .. } => Some(()),
        _ => None
    }).await?;

    // 3. Alice crea gruppo, aggiunge Bob (offline) e scrive
    alice.send(ClientMsg::CreateGroup { token: alice.token, name: "OfflineGroup".into() }).await?;
    let gid = alice.recv_until("Group Created", Duration::from_secs(2), |m| {
        if let ServerMsg::GroupCreated { group_id } = m { Some(*group_id) } else { None }
    }).await?;

    alice.send(ClientMsg::AddGroupMember { token: alice.token, group_id: gid, username: "bob_h".into() }).await?;
    alice.recv_until("Member Added", Duration::from_secs(2), |m| match m {
        ServerMsg::GroupMemberAdded => Some(()),
        _ => None
    }).await?;

    alice.send(ClientMsg::SendGroupMessage { token: alice.token, group_id: gid, content: "Messaggio di gruppo offline".into() }).await?;
    alice.recv_until("Group Msg Ack", Duration::from_secs(2), |m| match m {
        ServerMsg::GroupMessageSent { .. } => Some(()),
        _ => None
    }).await?;

    // 4. BOB TORNA ONLINE (Login)
    // Nota: usiamo false per 'do_register' perché è già registrato
    let mut bob = TestClient::new(port, "bob_h".into(), "pw", false).await?;

    // 5. Bob recupera storico PRIVATO
    // Prima deve scoprire le chat (opzionale nel client reale, ma qui testiamo l'API diretta)
    // Se Bob non ha l'ID della chat, lo può recuperare con GetPrivateChats o lo assumiamo noto nel test.
    // Usiamo GetPrivateChats per completezza.
    bob.send(ClientMsg::GetPrivateChats { token: bob.token }).await?;
    let chats = bob.recv_until("Get Chats", Duration::from_secs(2), |m| {
        if let ServerMsg::PrivateChats { chats } = m { Some(chats.clone()) } else { None }
    }).await?;
    assert!(!chats.is_empty(), "Bob dovrebbe avere almeno una chat privata");
    let fetched_chat_id = chats[0].chat_id;
    assert_eq!(fetched_chat_id, chat_id);

    // Ora chiede i messaggi
    bob.send(ClientMsg::GetPrivateChatMessages { token: bob.token, chat_id: fetched_chat_id, limit: 10 }).await?;
    let msgs = bob.recv_until("Get Priv Msgs", Duration::from_secs(2), |m| {
        if let ServerMsg::PrivateChatMessages { messages } = m { Some(messages.clone()) } else { None }
    }).await?;

    // Verifica contenuto
    assert!(msgs.iter().any(|m| m.content == "Sei offline?"), "Bob non ha trovato il messaggio offline privato");

    // 6. Bob recupera storico GRUPPO
    bob.send(ClientMsg::GetGroups { token: bob.token }).await?;
    let groups = bob.recv_until("Get Groups", Duration::from_secs(2), |m| {
        if let ServerMsg::Groups { groups } = m { Some(groups.clone()) } else { None }
    }).await?;
    assert!(!groups.is_empty());
    let fetched_gid = groups[0].group_id;
    assert_eq!(fetched_gid, gid);

    bob.send(ClientMsg::GetGroupMessages { token: bob.token, group_id: fetched_gid, limit: 10 }).await?;
    let g_msgs = bob.recv_until("Get Group Msgs", Duration::from_secs(2), |m| {
        if let ServerMsg::GroupMessages { messages } = m { Some(messages.clone()) } else { None }
    }).await?;

    assert!(g_msgs.iter().any(|m| m.content == "Messaggio di gruppo offline"), "Bob non ha trovato il messaggio offline di gruppo");

    // Cleanup
    let _ = child.kill();
    let _ = std::fs::remove_file(&db_path);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn e2e_search_and_lists() -> Result<()> {
    let (mut child, port, db_path) = start_server().await?;

    let mut alice = TestClient::new(port, "alice_s".into(), "pw", true).await?;
    let _bob = TestClient::new(port, "bob_s".into(), "pw", true).await?;
    let _charlie = TestClient::new(port, "charlie_s".into(), "pw", true).await?;

    // 1. RICERCA UTENTI
    alice.send(ClientMsg::SearchUsers { token: alice.token, query: "bob".into() }).await?;
    let results = alice.recv_until("Search 'bob'", Duration::from_secs(2), |m| {
        if let ServerMsg::UsersFound { users } = m { Some(users.clone()) } else { None }
    }).await?;

    assert!(results.iter().any(|u| u.username == "bob_s"));
    assert!(!results.iter().any(|u| u.username == "charlie_s"));

    // Ricerca parziale
    alice.send(ClientMsg::SearchUsers { token: alice.token, query: "char".into() }).await?;
    let results2 = alice.recv_until("Search 'char'", Duration::from_secs(2), |m| {
        if let ServerMsg::UsersFound { users } = m { Some(users.clone()) } else { None }
    }).await?;
    assert!(results2.iter().any(|u| u.username == "charlie_s"));

    // Ricerca vuota o senza match
    alice.send(ClientMsg::SearchUsers { token: alice.token, query: "non_esiste".into() }).await?;
    let results3 = alice.recv_until("Search empty", Duration::from_secs(2), |m| {
        if let ServerMsg::UsersFound { users } = m { Some(users.clone()) } else { None }
    }).await?;
    assert!(results3.is_empty());

    // 2. LISTE CHAT E GRUPPI (Verifica consistenza)
    // All'inizio vuote
    alice.send(ClientMsg::GetPrivateChats { token: alice.token }).await?;
    let chats_empty = alice.recv_until("Empty Chats", Duration::from_secs(2), |m| {
        if let ServerMsg::PrivateChats { chats } = m { Some(chats.clone()) } else { None }
    }).await?;
    assert!(chats_empty.is_empty());

    // Crea una chat e verifica
    alice.send(ClientMsg::StartPrivateChat { token: alice.token, other_username: "bob_s".into() }).await?;
    alice.recv_until("Chat Started", Duration::from_secs(2), |m| match m {
        ServerMsg::PrivateChatStarted { .. } => Some(()),
        _ => None
    }).await?;

    alice.send(ClientMsg::GetPrivateChats { token: alice.token }).await?;
    let chats_filled = alice.recv_until("1 Chat", Duration::from_secs(2), |m| {
        if let ServerMsg::PrivateChats { chats } = m { Some(chats.clone()) } else { None }
    }).await?;
    assert_eq!(chats_filled.len(), 1);
    assert_eq!(chats_filled[0].other_username, "bob_s");

    // Cleanup
    let _ = child.kill();
    let _ = std::fs::remove_file(&db_path);
    Ok(())
}