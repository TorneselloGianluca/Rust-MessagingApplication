use eframe::egui::{self, Align, Layout, RichText};
use std::sync::mpsc;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;
use shared::{PrivateChatInfo, GroupInfo, MessageInfo, UserInfo};
use std::time::Instant;

use crate::views;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Home,
    Login,
    Register,
    ChatList,
    PrivateChat(usize),
    GroupChat(usize),
}

impl Default for View { fn default() -> Self { View::Home } }

#[derive(Default)]
pub struct App {
    pub view: View,
    pub server_addr: String,
    pub login_username: String,
    pub login_password: String,
    pub reg_username: String,
    pub reg_password: String,
    pub session_token: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub search_query: String,
    pub search_results: Vec<UserInfo>,
    pub last_search_completed: bool,
    pub search_last_edit: Option<Instant>,
    pub private_chats: Vec<PrivateChatInfo>,
    pub groups: Vec<GroupInfo>,
    pub chats_loaded: bool,
    pub new_group_name: String,
    pub add_member_username: String,
    pub current_messages: Vec<MessageInfo>,
    pub current_group_members: Vec<UserInfo>,
    pub message_input: String,
    pub status: String,
    pub is_loading: bool,
    pub rt: Option<Runtime>,
    pub rx_result: Option<mpsc::Receiver<AppResult>>,
    pub tx_result: Option<mpsc::Sender<AppResult>>,
    pub show_group_info: bool,
}

#[derive(Debug)]
pub enum AppResult {
    LoginSuccess { token: Uuid, user_id: Uuid, username: String },
    RegisterSuccess,
    SearchResults { users: Vec<UserInfo> },
    PrivateChatsLoaded { chats: Vec<PrivateChatInfo> },
    GroupsLoaded { groups: Vec<GroupInfo> },
    GroupMembersLoaded { members: Vec<UserInfo> },
    MessagesLoaded { messages: Vec<MessageInfo> },
    PrivateChatStarted { chat_id: Uuid },
    GroupCreated { group_id: Uuid },
    MessageSent,
    MemberAdded,
    Error { message: String },

    PushGroupListUpdated,
    PushPrivateChatListUpdated,
    PushNewMessage { message: MessageInfo, chat_id: Option<Uuid>, group_id: Option<Uuid> },
}

impl App {
    pub fn reset_status(&mut self) {
        self.status.clear();
        self.is_loading = false;
    }

    pub fn nav(&mut self, v: View) {
        self.view = v;
        self.reset_status();
        self.show_group_info = false;
        self.search_results.clear();
        self.last_search_completed = false;
        self.search_last_edit = None;

        if !matches!(v, View::PrivateChat(_) | View::GroupChat(_) | View::ChatList) {
            self.current_messages.clear();
            self.current_group_members.clear();
        }
        if matches!(v, View::ChatList) {
            self.current_group_members.clear();
        }
    }

    pub fn is_logged_in(&self) -> bool { self.session_token.is_some() }

    pub fn logout(&mut self) {
        self.session_token = None;
        self.user_id = None;
        self.username = None;
        self.private_chats.clear();
        self.groups.clear();
        self.current_messages.clear();
        self.current_group_members.clear();
        self.search_results.clear();
        self.last_search_completed = false;
        self.chats_loaded = false;
        let (tx, rx) = mpsc::channel();
        self.rx_result = Some(rx);
        self.tx_result = Some(tx);
        self.nav(View::Home);
    }

    pub fn get_tx(&mut self) -> mpsc::Sender<AppResult> {
        if self.tx_result.is_none() {
            let (tx, rx) = mpsc::channel();
            self.tx_result = Some(tx);
            self.rx_result = Some(rx);
        }
        self.tx_result.as_ref().unwrap().clone()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.rt.is_none() {
            self.rt = Some(Runtime::new().expect("tokio runtime"));
            let (tx, rx) = mpsc::channel();
            self.tx_result = Some(tx);
            self.rx_result = Some(rx);
        }
        if self.server_addr.is_empty() { self.server_addr = "127.0.0.1:7878".into(); }

        egui::TopBottomPanel::top("global_top_panel").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading(RichText::new("Ruggine").strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let username_display = self.username.clone();
                    if let Some(username) = username_display {
                        if ui.add_enabled(!self.is_loading, egui::Button::new("Logout")).clicked() { self.logout(); }
                        ui.label(format!("Connesso come: {}", username));
                    }
                });
            });
        });

        if matches!(self.view, View::PrivateChat(_) | View::GroupChat(_) | View::ChatList) {
            match self.view {
                View::ChatList => views::chat_list::show(self, ctx),
                View::PrivateChat(idx) => views::private_chat::show(self, ctx, idx),
                View::GroupChat(idx) => views::group_chat::show(self, ctx, idx),
                _ => {}
            }
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.add_space(16.0);
                    match self.view {
                        View::Home => views::home::show(self, ui),
                        View::Login => views::login::show(self, ui, ctx),
                        View::Register => views::register::show(self, ui, ctx),
                        _ => {}
                    }
                    ui.add_space(ui.available_height() - 30.0);
                    ui.separator();
                    if self.is_loading { ui.horizontal(|ui| { ui.spinner(); ui.label(&self.status); }); }
                    else if !self.status.is_empty() { ui.label(&self.status); }
                });
            });
        }

        let mut events = Vec::new();
        if let Some(rx) = &self.rx_result {
            while let Ok(result) = rx.try_recv() { events.push(result); }
        }

        for result in events {
            match result {
                AppResult::LoginSuccess { token, user_id, username } => {
                    self.is_loading = false;
                    self.session_token = Some(token);
                    self.user_id = Some(user_id);
                    self.username = Some(username);
                    self.status = "✅ Login effettuato!".into();
                    self.nav(View::ChatList);
                    let addr = self.server_addr.clone();
                    let tx = self.get_tx();
                    let rt = self.rt.as_ref().unwrap().handle().clone();
                    rt.spawn(async move { crate::net::listen_background(addr, token, tx).await; });
                }
                AppResult::PushGroupListUpdated => { if let Some(token) = self.session_token { self.load_groups(ctx, token, false); } }
                AppResult::PushPrivateChatListUpdated => { if let Some(token) = self.session_token { self.load_private_chats(ctx, token, false); } }

                AppResult::PushNewMessage { message, chat_id: msg_chat_id, group_id: msg_group_id } => {
                    let should_add = match self.view {
                        View::PrivateChat(idx) => {
                            if let Some(chat) = self.private_chats.get(idx) {
                                // *** FIX APPLICATO QUI ***
                                // Controlliamo se il messaggio appartiene a questa chat controllando:
                                // 1. Se l'ID chat corrisponde (come prima)
                                // 2. OPPURE se il mittente è l'utente con cui stiamo parlando (corregge bug messaggi in arrivo)
                                Some(chat.chat_id) == msg_chat_id || message.sender_id == chat.chat_id
                            } else { false }
                        },
                        View::GroupChat(idx) => {
                            if let Some(group) = self.groups.get(idx) {
                                Some(group.group_id) == msg_group_id
                            } else { false }
                        },
                        _ => false
                    };

                    if should_add {
                        let exists = self.current_messages.iter().any(|m| m.message_id == message.message_id);
                        if !exists { self.current_messages.push(message); }
                    }
                }

                AppResult::RegisterSuccess => { self.is_loading = false; self.nav(View::Home); self.status = "Registrato! Fai login.".into(); }
                AppResult::SearchResults { users } => { self.is_loading = false; self.search_results = users; self.last_search_completed = true; }
                AppResult::PrivateChatsLoaded { chats } => {
                    self.is_loading = false;
                    self.status.clear();
                    self.private_chats = chats;
                    if self.groups.is_empty() && self.session_token.is_some() { if let Some(t) = self.session_token { self.load_groups(ctx, t, true); } }
                }
                AppResult::GroupsLoaded { groups } => {
                    self.is_loading = false;
                    self.status.clear();
                    self.groups = groups;
                }
                AppResult::GroupMembersLoaded { members } => { self.is_loading = false; self.current_group_members = members; }
                AppResult::MessagesLoaded { messages } => {
                    self.is_loading = false;
                    let mut r = messages; r.reverse();
                    self.current_messages = r;
                    if let View::GroupChat(idx) = self.view {
                        if self.current_group_members.is_empty() { if let (Some(t), Some(g)) = (self.session_token, self.groups.get(idx)) { self.load_group_members(ctx, t, g.group_id); } }
                    }
                }
                AppResult::PrivateChatStarted { chat_id } => {
                    self.is_loading = false;
                    if let Some(t) = self.session_token { self.load_private_chats(ctx, t, true); }
                    self.status = format!("Chat avviata: {}", chat_id);
                }
                AppResult::GroupCreated { group_id } => {
                    self.is_loading = false;
                    self.new_group_name.clear();
                    if let Some(t) = self.session_token { self.load_groups(ctx, t, true); }
                    self.status = format!("Gruppo creato: {}", group_id);
                }
                AppResult::MessageSent => {},
                AppResult::MemberAdded => {
                    self.is_loading = false;
                    self.add_member_username.clear();
                    if let View::GroupChat(idx) = self.view { if let (Some(t), Some(g)) = (self.session_token, self.groups.get(idx)) { self.load_group_members(ctx, t, g.group_id); } }
                },
                AppResult::Error { message } => { self.is_loading = false; self.status = format!("Errore: {}", message); }
            }
        }
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}

impl App {
    pub fn load_private_chats(&mut self, ctx: &egui::Context, token: Uuid, show_loading: bool) {
        let tx = self.get_tx();
        if show_loading { self.is_loading = true; self.status = "Loading chats...".into(); }
        let addr = self.server_addr.clone();
        let rt = self.rt.as_ref().unwrap().handle().clone();
        let ctx2 = ctx.clone();
        rt.spawn(async move {
            let res = crate::net::get_private_chats(&addr, token).await;
            let _ = tx.send(res);
            ctx2.request_repaint();
        });
    }
    pub fn load_groups(&mut self, ctx: &egui::Context, token: Uuid, show_loading: bool) {
        let tx = self.get_tx();
        if show_loading { self.is_loading = true; self.status = "Loading groups...".into(); }
        let addr = self.server_addr.clone();
        let rt = self.rt.as_ref().unwrap().handle().clone();
        let ctx2 = ctx.clone();
        rt.spawn(async move {
            let res = crate::net::get_groups(&addr, token).await;
            let _ = tx.send(res);
            ctx2.request_repaint();
        });
    }
    pub fn load_private_messages(&mut self, ctx: &egui::Context, token: Uuid, chat_id: Uuid, show_loading: bool) {
        let tx = self.get_tx();
        if show_loading { self.is_loading = true; }
        let addr = self.server_addr.clone();
        let rt = self.rt.as_ref().unwrap().handle().clone();
        let ctx2 = ctx.clone();
        rt.spawn(async move {
            let res = crate::net::get_private_messages(&addr, token, chat_id, 50).await;
            let _ = tx.send(res);
            ctx2.request_repaint();
        });
    }
    pub fn load_group_messages(&mut self, ctx: &egui::Context, token: Uuid, group_id: Uuid, show_loading: bool) {
        let tx = self.get_tx();
        if show_loading { self.is_loading = true; }
        let addr = self.server_addr.clone();
        let rt = self.rt.as_ref().unwrap().handle().clone();
        let ctx2 = ctx.clone();
        rt.spawn(async move {
            let res = crate::net::get_group_messages(&addr, token, group_id, 50).await;
            let _ = tx.send(res);
            ctx2.request_repaint();
        });
    }
    pub fn load_group_members(&mut self, ctx: &egui::Context, token: Uuid, group_id: Uuid) {
        let tx = self.get_tx();
        let addr = self.server_addr.clone();
        let rt = self.rt.as_ref().unwrap().handle().clone();
        let ctx2 = ctx.clone();
        rt.spawn(async move {
            let res = crate::net::get_group_members(&addr, token, group_id).await;
            let _ = tx.send(res);
            ctx2.request_repaint();
        });
    }
}