use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use shared::{ClientMsg, ServerMsg};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use uuid::Uuid;
use crate::app::AppResult;

// Helper (Invariato)
async fn send_and_receive(addr: &str, msg: ClientMsg) -> Result<ServerMsg> {
    let stream = TcpStream::connect(addr).await?;
    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());
    framed.send(serde_json::to_vec(&msg)?.into()).await?;
    if let Some(Ok(bytes)) = framed.next().await {
        Ok(serde_json::from_slice(&bytes)?)
    } else {
        Ok(ServerMsg::Error { message: "Nessuna risposta dal server".into() })
    }
}

// *** LISTENER (MODIFICATO) ***
pub async fn listen_background(addr: String, token: Uuid, tx_app: std::sync::mpsc::Sender<AppResult>) {
    let stream = match TcpStream::connect(&addr).await {
        Ok(s) => s,
        Err(e) => { eprintln!("Listener error: {}", e); return; }
    };
    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

    let msg = ClientMsg::Listen { token };
    let _ = framed.send(serde_json::to_vec(&msg).unwrap().into()).await;

    while let Some(Ok(bytes)) = framed.next().await {
        if let Ok(server_msg) = serde_json::from_slice::<ServerMsg>(&bytes) {
            match server_msg {
                ServerMsg::PushGroupUpdated => { let _ = tx_app.send(AppResult::PushGroupListUpdated); },
                ServerMsg::PushPrivateChatListUpdated => { let _ = tx_app.send(AppResult::PushPrivateChatListUpdated); },

                // *** PUSH MESSAGGIO (Ora con ID chat/gruppo) ***
                ServerMsg::PushNewMessage { message, chat_id, group_id } => {
                    let _ = tx_app.send(AppResult::PushNewMessage { message, chat_id, group_id });
                },
                _ => {}
            }
        }
    }
    eprintln!("Listener closed");
}

// Wrapper standard (Tutti invariati rispetto a prima)
pub async fn login(addr: &str, username: &str, password: &str) -> AppResult {
    let msg = ClientMsg::Login { username: username.into(), password: password.into() };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::LoginOk { session_token, user_id, username }) => AppResult::LoginSuccess { token: session_token, user_id, username },
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn register(addr: &str, username: &str, password: &str) -> AppResult {
    let msg = ClientMsg::Register { username: username.into(), password: password.into() };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::Registered { .. }) => AppResult::RegisterSuccess,
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn search_users(addr: &str, token: Uuid, query: &str) -> AppResult {
    let msg = ClientMsg::SearchUsers { token, query: query.into() };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::UsersFound { users }) => AppResult::SearchResults { users },
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn start_private_chat(addr: &str, token: Uuid, other_username: &str) -> AppResult {
    let msg = ClientMsg::StartPrivateChat { token, other_username: other_username.into() };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::PrivateChatStarted { chat_id }) => AppResult::PrivateChatStarted { chat_id },
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn get_private_chats(addr: &str, token: Uuid) -> AppResult {
    let msg = ClientMsg::GetPrivateChats { token };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::PrivateChats { chats }) => AppResult::PrivateChatsLoaded { chats },
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn get_private_messages(addr: &str, token: Uuid, chat_id: Uuid, limit: u32) -> AppResult {
    let msg = ClientMsg::GetPrivateChatMessages { token, chat_id, limit };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::PrivateChatMessages { messages }) => AppResult::MessagesLoaded { messages },
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn send_private_message(addr: &str, token: Uuid, chat_id: Uuid, content: &str) -> AppResult {
    let msg = ClientMsg::SendPrivateMessage { token, chat_id, content: content.into() };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::PrivateMessageSent { .. }) => AppResult::MessageSent,
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn create_group(addr: &str, token: Uuid, name: &str) -> AppResult {
    let msg = ClientMsg::CreateGroup { token, name: name.into() };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::GroupCreated { group_id }) => AppResult::GroupCreated { group_id },
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn add_group_member(addr: &str, token: Uuid, group_id: Uuid, username: &str) -> AppResult {
    let msg = ClientMsg::AddGroupMember { token, group_id, username: username.into() };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::GroupMemberAdded) => AppResult::MemberAdded,
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn get_groups(addr: &str, token: Uuid) -> AppResult {
    let msg = ClientMsg::GetGroups { token };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::Groups { groups }) => AppResult::GroupsLoaded { groups },
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn get_group_members(addr: &str, token: Uuid, group_id: Uuid) -> AppResult {
    let msg = ClientMsg::GetGroupMembers { token, group_id };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::GroupMembers { members }) => AppResult::GroupMembersLoaded { members },
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn get_group_messages(addr: &str, token: Uuid, group_id: Uuid, limit: u32) -> AppResult {
    let msg = ClientMsg::GetGroupMessages { token, group_id, limit };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::GroupMessages { messages }) => AppResult::MessagesLoaded { messages },
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}

pub async fn send_group_message(addr: &str, token: Uuid, group_id: Uuid, content: &str) -> AppResult {
    let msg = ClientMsg::SendGroupMessage { token, group_id, content: content.into() };
    match send_and_receive(addr, msg).await {
        Ok(ServerMsg::GroupMessageSent { .. }) => AppResult::MessageSent,
        Ok(ServerMsg::Error { message }) => AppResult::Error { message },
        _ => AppResult::Error { message: "Err".into() },
    }
}