use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMsg {
    Register { username: String, password: String },
    Login { username: String, password: String },
    Listen { token: Uuid },

    SearchUsers { token: Uuid, query: String },

    StartPrivateChat { token: Uuid, other_username: String },
    GetPrivateChats { token: Uuid },
    GetPrivateChatMessages { token: Uuid, chat_id: Uuid, limit: u32 },
    SendPrivateMessage { token: Uuid, chat_id: Uuid, content: String },

    CreateGroup { token: Uuid, name: String },
    AddGroupMember { token: Uuid, group_id: Uuid, username: String },
    GetGroups { token: Uuid },
    GetGroupMembers { token: Uuid, group_id: Uuid },
    GetGroupMessages { token: Uuid, group_id: Uuid, limit: u32 },
    SendGroupMessage { token: Uuid, group_id: Uuid, content: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerMsg {
    Registered { user_id: Uuid },
    LoginOk { session_token: Uuid, user_id: Uuid, username: String },

    // *** NOTIFICHE PUSH (AGGIORNATE) ***
    PushNewMessage {
        message: MessageInfo,
        chat_id: Option<Uuid>,  // Presente se è una chat privata
        group_id: Option<Uuid>, // Presente se è un gruppo
    },
    PushGroupUpdated,
    PushPrivateChatListUpdated,

    UsersFound { users: Vec<UserInfo> },

    PrivateChatStarted { chat_id: Uuid },
    PrivateChats { chats: Vec<PrivateChatInfo> },
    PrivateChatMessages { messages: Vec<MessageInfo> },
    PrivateMessageSent { message_id: Uuid },

    GroupCreated { group_id: Uuid },
    GroupMemberAdded,
    Groups { groups: Vec<GroupInfo> },
    GroupMembers { members: Vec<UserInfo> },
    GroupMessages { messages: Vec<MessageInfo> },
    GroupMessageSent { message_id: Uuid },

    Error { message: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrivateChatInfo {
    pub chat_id: Uuid,
    pub other_user_id: Uuid,
    pub other_username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupInfo {
    pub group_id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageInfo {
    pub message_id: Uuid,
    pub sender_id: Uuid,
    pub sender_username: String,
    pub content: String,
    pub sent_at: i64,
}