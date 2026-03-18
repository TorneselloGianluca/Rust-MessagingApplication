use crate::{
    auth,
    errors::{AppError, AppResult},
    storage::SqliteStorage,
    PeerMap,
};
use shared::{UserInfo, PrivateChatInfo, GroupInfo, MessageInfo, ServerMsg};
use uuid::Uuid;

// Auth & Search
pub fn handle_register(db: &SqliteStorage, username: String, password: String) -> AppResult<Uuid> {
    if username.trim().is_empty() || password.trim().is_empty() { return Err(AppError::Validation("Input vuoti".into())); }
    let hash = auth::hash_password(&password)?;
    db.insert_user(&username, &hash)
}

pub fn handle_login(db: &SqliteStorage, username: String, password: String) -> AppResult<(Uuid, Uuid, String)> {
    let Some(stored) = db.get_pwd_hash(&username)? else { return Err(AppError::BadCredentials); };
    if !auth::verify_password(&password, &stored)? { return Err(AppError::BadCredentials); }
    let token = db.insert_session(&username, 86400)?;
    let user_id = db.get_user_id(&username)?.ok_or(AppError::BadCredentials)?;
    Ok((token, user_id, username))
}

pub fn handle_search_users(db: &SqliteStorage, token: Uuid, query: String) -> AppResult<Vec<UserInfo>> {
    // *** MODIFICA: Otteniamo lo user_id invece di ignorarlo ***
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;

    // *** MODIFICA: Passiamo user_id a search_users ***
    let users = db.search_users(&user_id, &query, 20)?;

    Ok(users.into_iter().map(|(id, name)| UserInfo { user_id: id, username: name }).collect())
}

// PRIVATE CHAT
pub fn handle_start_private_chat(db: &SqliteStorage, peers: &PeerMap, token: Uuid, other_username: String) -> AppResult<Uuid> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    let other_id = db.get_user_id(&other_username)?.ok_or(AppError::Validation("Utente non trovato".into()))?;
    if user_id == other_id { return Err(AppError::Validation("No chat con te stesso".into())); }

    let chat_id = db.create_private_chat(&user_id, &other_id)?;

    if let Some(tx) = peers.lock().unwrap().get(&other_id) {
        let _ = tx.send(ServerMsg::PushPrivateChatListUpdated);
    }
    Ok(chat_id)
}

pub fn handle_get_private_chats(db: &SqliteStorage, token: Uuid) -> AppResult<Vec<PrivateChatInfo>> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    let chats = db.get_user_private_chats(&user_id)?;
    Ok(chats.into_iter().map(|(cid, oid, oname)| PrivateChatInfo { chat_id: cid, other_user_id: oid, other_username: oname }).collect())
}

pub fn handle_get_private_chat_messages(db: &SqliteStorage, token: Uuid, chat_id: Uuid, limit: u32) -> AppResult<Vec<MessageInfo>> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    if !db.is_user_in_private_chat(&user_id, &chat_id)? { return Err(AppError::Validation("Accesso negato".into())); }
    let msgs = db.get_private_chat_messages(&chat_id, limit)?;
    Ok(msgs.into_iter().map(|(mid, sid, sname, c, sat)| MessageInfo { message_id: mid, sender_id: sid, sender_username: sname, content: c, sent_at: sat }).collect())
}

pub fn handle_send_private_message(db: &SqliteStorage, peers: &PeerMap, token: Uuid, chat_id: Uuid, content: String) -> AppResult<Uuid> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    if content.trim().is_empty() { return Err(AppError::Validation("Messaggio vuoto".into())); }
    if !db.is_user_in_private_chat(&user_id, &chat_id)? { return Err(AppError::Validation("Accesso negato".into())); }

    let msg_id = db.insert_message(&user_id, &content, Some(&chat_id), None)?;
    let username = db.get_username(&user_id)?.unwrap_or("?".into());

    let msg_info = MessageInfo {
        message_id: msg_id,
        sender_id: user_id,
        sender_username: username,
        content,
        sent_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
    };

    if let Some((u1, u2)) = db.get_private_chat_members(&chat_id)? {
        let other_id = if u1 == user_id { u2 } else { u1 };
        if let Some(tx) = peers.lock().unwrap().get(&other_id) {
            let _ = tx.send(ServerMsg::PushNewMessage {
                message: msg_info,
                chat_id: Some(chat_id),
                group_id: None
            });
        }
    }
    Ok(msg_id)
}

// GRUPPI
pub fn handle_create_group(db: &SqliteStorage, token: Uuid, name: String) -> AppResult<Uuid> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    if name.trim().is_empty() { return Err(AppError::Validation("Nome vuoto".into())); }
    let gid = db.create_group(&name, &user_id)?;
    Ok(gid)
}

pub fn handle_add_group_member(db: &SqliteStorage, peers: &PeerMap, token: Uuid, group_id: Uuid, username: String) -> AppResult<()> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    if !db.is_user_in_group(&user_id, &group_id)? { return Err(AppError::Validation("Non membro".into())); }
    let target = db.get_user_id(&username)?.ok_or(AppError::Validation("Utente non trovato".into()))?;
    if target == user_id { return Err(AppError::Validation("Auto-aggiunta vietata".into())); }
    if db.is_user_in_group(&target, &group_id)? { return Err(AppError::Validation("Già membro".into())); }

    db.add_group_member(&group_id, &target)?;

    if let Some(tx) = peers.lock().unwrap().get(&target) {
        let _ = tx.send(ServerMsg::PushGroupUpdated);
    }
    Ok(())
}

pub fn handle_get_groups(db: &SqliteStorage, token: Uuid) -> AppResult<Vec<GroupInfo>> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    let groups = db.get_user_groups(&user_id)?;
    Ok(groups.into_iter().map(|(gid, name)| GroupInfo { group_id: gid, name }).collect())
}

pub fn handle_get_group_members(db: &SqliteStorage, token: Uuid, group_id: Uuid) -> AppResult<Vec<UserInfo>> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    if !db.is_user_in_group(&user_id, &group_id)? { return Err(AppError::Validation("Accesso negato".into())); }
    let members = db.get_group_members(&group_id)?;
    Ok(members.into_iter().map(|(uid, name)| UserInfo { user_id: uid, username: name }).collect())
}

pub fn handle_get_group_messages(db: &SqliteStorage, token: Uuid, group_id: Uuid, limit: u32) -> AppResult<Vec<MessageInfo>> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    if !db.is_user_in_group(&user_id, &group_id)? { return Err(AppError::Validation("Accesso negato".into())); }
    let msgs = db.get_group_messages(&group_id, limit)?;
    Ok(msgs.into_iter().map(|(mid, sid, sname, c, sat)| MessageInfo { message_id: mid, sender_id: sid, sender_username: sname, content: c, sent_at: sat }).collect())
}

pub fn handle_send_group_message(db: &SqliteStorage, peers: &PeerMap, token: Uuid, group_id: Uuid, content: String) -> AppResult<Uuid> {
    let user_id = db.validate_session(&token)?.ok_or(AppError::BadCredentials)?;
    if content.trim().is_empty() { return Err(AppError::Validation("Messaggio vuoto".into())); }
    if !db.is_user_in_group(&user_id, &group_id)? { return Err(AppError::Validation("Accesso negato".into())); }

    let msg_id = db.insert_message(&user_id, &content, None, Some(&group_id))?;
    let username = db.get_username(&user_id)?.unwrap_or("?".into());

    let msg_info = MessageInfo {
        message_id: msg_id,
        sender_id: user_id,
        sender_username: username,
        content,
        sent_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
    };

    let members = db.get_group_members(&group_id)?;
    let peers_guard = peers.lock().unwrap();
    for (member_id, _) in members {
        if member_id != user_id {
            if let Some(tx) = peers_guard.get(&member_id) {
                let _ = tx.send(ServerMsg::PushNewMessage {
                    message: msg_info.clone(),
                    chat_id: None,
                    group_id: Some(group_id)
                });
            }
        }
    }
    Ok(msg_id)
}