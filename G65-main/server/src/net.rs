use futures_util::{SinkExt, StreamExt};
use shared::{ClientMsg, ServerMsg};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio::sync::mpsc;
use uuid::Uuid;
use crate::{
    handlers,
    storage::SqliteStorage,
    errors::AppError,
    PeerMap,
};

pub async fn serve_connection(sock: TcpStream, db_path: &'static str, peers: PeerMap) {
    let mut framed = Framed::new(sock, LengthDelimitedCodec::new());
    let db = match SqliteStorage::new(db_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Errore apertura DB: {}", e);
            return;
        }
    };

    let (tx_push, mut rx_push) = mpsc::unbounded_channel::<ServerMsg>();
    let mut my_user_id: Option<Uuid> = None;

    loop {
        tokio::select! {
            result = framed.next() => match result {
                Some(Ok(bytes)) => {
                    let msg = match serde_json::from_slice::<ClientMsg>(&bytes) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("JSON err: {}", e);
                            continue;
                        }
                    };

                    if let ClientMsg::Listen { token } = &msg {
                         match db.validate_session(token) {
                            Ok(Some(user_id)) => {
                                println!("Utente {} in ascolto push", user_id);
                                peers.lock().unwrap().insert(user_id, tx_push.clone());
                                my_user_id = Some(user_id);
                            }
                            _ => {
                                let _ = send(&mut framed, &ServerMsg::Error { message: "Sessione non valida".into() }).await;
                                break;
                            }
                         }
                         continue;
                    }

                    let resp = handle_client_msg(&msg, &db, &peers);
                    let _ = send(&mut framed, &resp).await;
                }
                _ => break,
            },
            Some(push_msg) = rx_push.recv() => {
                let _ = send(&mut framed, &push_msg).await;
            }
        }
    }

    // *** BUG FIX: DISCONNESSIONE SICURA ***
    if let Some(uid) = my_user_id {
        let mut peers_guard = peers.lock().unwrap();

        // Controlliamo se c'è ancora un canale per questo utente
        if let Some(stored_tx) = peers_guard.get(&uid) {
            // Rimuoviamo l'utente dalla mappa SOLO SE il canale memorizzato
            // è esattamente quello di QUESTA connessione che si sta chiudendo.
            // Se è diverso, significa che l'utente si è riconnesso altrove
            // e non dobbiamo cancellare la sua nuova sessione.
            if stored_tx.same_channel(&tx_push) {
                peers_guard.remove(&uid);
                println!("Utente {} disconnesso (sessione pulita)", uid);
            } else {
                println!("Utente {} disconnesso (sessione preservata, nuova connessione attiva)", uid);
            }
        }
    }
}

fn handle_client_msg(msg: &ClientMsg, db: &SqliteStorage, peers: &PeerMap) -> ServerMsg {
    match msg {
        ClientMsg::Register { username, password } => {
            match handlers::handle_register(db, username.clone(), password.clone()) {
                Ok(id) => ServerMsg::Registered { user_id: id },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::Login { username, password } => {
            match handlers::handle_login(db, username.clone(), password.clone()) {
                Ok((token, user_id, username)) => ServerMsg::LoginOk {
                    session_token: token,
                    user_id,
                    username
                },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::SearchUsers { token, query } => {
            match handlers::handle_search_users(db, *token, query.clone()) {
                Ok(users) => ServerMsg::UsersFound { users },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::StartPrivateChat { token, other_username } => {
            match handlers::handle_start_private_chat(db, peers, *token, other_username.clone()) {
                Ok(chat_id) => ServerMsg::PrivateChatStarted { chat_id },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::GetPrivateChats { token } => {
            match handlers::handle_get_private_chats(db, *token) {
                Ok(chats) => ServerMsg::PrivateChats { chats },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::GetPrivateChatMessages { token, chat_id, limit } => {
            match handlers::handle_get_private_chat_messages(db, *token, *chat_id, *limit) {
                Ok(messages) => ServerMsg::PrivateChatMessages { messages },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::SendPrivateMessage { token, chat_id, content } => {
            match handlers::handle_send_private_message(db, peers, *token, *chat_id, content.clone()) {
                Ok(message_id) => ServerMsg::PrivateMessageSent { message_id },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::CreateGroup { token, name } => {
            match handlers::handle_create_group(db, *token, name.clone()) {
                Ok(group_id) => ServerMsg::GroupCreated { group_id },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::AddGroupMember { token, group_id, username } => {
            match handlers::handle_add_group_member(db, peers, *token, *group_id, username.clone()) {
                Ok(()) => ServerMsg::GroupMemberAdded,
                Err(e) => map_error(e),
            }
        }
        ClientMsg::GetGroups { token } => {
            match handlers::handle_get_groups(db, *token) {
                Ok(groups) => ServerMsg::Groups { groups },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::GetGroupMembers { token, group_id } => {
            match handlers::handle_get_group_members(db, *token, *group_id) {
                Ok(members) => ServerMsg::GroupMembers { members },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::GetGroupMessages { token, group_id, limit } => {
            match handlers::handle_get_group_messages(db, *token, *group_id, *limit) {
                Ok(messages) => ServerMsg::GroupMessages { messages },
                Err(e) => map_error(e),
            }
        }
        ClientMsg::SendGroupMessage { token, group_id, content } => {
            match handlers::handle_send_group_message(db, peers, *token, *group_id, content.clone()) {
                Ok(message_id) => ServerMsg::GroupMessageSent { message_id },
                Err(e) => map_error(e),
            }
        }
        _ => ServerMsg::Error { message: "Messaggio non gestito".into() }
    }
}

fn map_error(e: AppError) -> ServerMsg {
    use AppError::*;
    let message = match e {
        UserExists => "Username già in uso".into(),
        BadCredentials => "Credenziali errate".into(),
        Validation(m) => m,
        other => other.to_string(),
    };
    ServerMsg::Error { message }
}

async fn send(
    framed: &mut Framed<TcpStream, LengthDelimitedCodec>,
    msg: &ServerMsg,
) -> Result<(), std::io::Error> {
    let bytes = serde_json::to_vec(msg).expect("serialize");
    framed.send(bytes.into()).await
}