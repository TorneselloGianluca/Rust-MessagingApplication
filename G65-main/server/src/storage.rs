use crate::errors::{AppError, AppResult};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

pub struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    pub fn new(db_path: &str) -> AppResult<Self> {
        let conn = Connection::open(db_path)?;

        // =================================================================
        // CONFIGURAZIONE BILANCIATA (Performance vs Consistenza)
        // =================================================================
        
        // 1. WAL Mode: Necessario per supportare 100+ utenti senza blocchi continui.
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // 2. Busy Timeout: Se il DB è occupato, aspetta 5s (CRUCIALE per evitare "database is locked").
        conn.pragma_update(None, "busy_timeout", 5000)?;

        // 3. Synchronous: Torniato a FULL (o NORMAL). 
        // 'NORMAL' causava la race condition "Credenziali Errate" su macchine veloci.
        // 'FULL' garantisce che dopo la Register, l'utente esista davvero per il Login.
        conn.pragma_update(None, "synchronous", "FULL")?;

        // =================================================================

        Ok(Self { conn })
    }

    pub fn init(db_path: &str) -> AppResult<()> {
        let conn = Connection::open(db_path)?;
        // Timeout anche in init per sicurezza
        let _ = conn.pragma_update(None, "busy_timeout", 5000);

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                pwd_hash TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS private_chats (
                id TEXT PRIMARY KEY,
                user1_id TEXT NOT NULL,
                user2_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (user1_id) REFERENCES users(id),
                FOREIGN KEY (user2_id) REFERENCES users(id),
                UNIQUE(user1_id, user2_id)
            );
            CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                creator_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (creator_id) REFERENCES users(id)
            );
            CREATE TABLE IF NOT EXISTS group_members (
                group_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                joined_at INTEGER NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(id),
                FOREIGN KEY (user_id) REFERENCES users(id),
                PRIMARY KEY (group_id, user_id)
            );
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                sender_id TEXT NOT NULL,
                private_chat_id TEXT,
                group_id TEXT,
                content TEXT NOT NULL,
                sent_at INTEGER NOT NULL,
                FOREIGN KEY (sender_id) REFERENCES users(id),
                FOREIGN KEY (private_chat_id) REFERENCES private_chats(id),
                FOREIGN KEY (group_id) REFERENCES groups(id),
                CHECK ((private_chat_id IS NULL) != (group_id IS NULL))
            );
            CREATE INDEX IF NOT EXISTS idx_messages_private ON messages(private_chat_id, sent_at);
            CREATE INDEX IF NOT EXISTS idx_messages_group ON messages(group_id, sent_at);
        "#,
        )?;
        Ok(())
    }

    // UTENTI
    pub fn insert_user(&self, username: &str, pwd_hash: &str) -> AppResult<Uuid> {
        let id = Uuid::new_v4();
        match self.conn.execute("INSERT INTO users(id, username, pwd_hash) VALUES (?1, ?2, ?3)", params![id.to_string(), username, pwd_hash]) {
            Ok(_) => Ok(id),
            Err(e) if is_unique_violation(&e) => Err(AppError::UserExists),
            Err(e) => Err(AppError::Db(e)),
        }
    }

    pub fn get_pwd_hash(&self, username: &str) -> AppResult<Option<String>> {
        Ok(self.conn.query_row("SELECT pwd_hash FROM users WHERE username = ?1", [username], |r| r.get(0)).optional()?)
    }

    pub fn get_user_id(&self, username: &str) -> AppResult<Option<Uuid>> {
        let row = self.conn.query_row("SELECT id FROM users WHERE username = ?1", [username], |r| {
            let s: String = r.get(0)?;
            Ok(Uuid::parse_str(&s).unwrap())
        }).optional()?;
        Ok(row)
    }

    pub fn get_username(&self, user_id: &Uuid) -> AppResult<Option<String>> {
        Ok(self.conn.query_row("SELECT username FROM users WHERE id = ?1", [user_id.to_string()], |r| r.get(0)).optional()?)
    }

    pub fn search_users(&self, my_id: &Uuid, query: &str, limit: u32) -> AppResult<Vec<(Uuid, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, username FROM users WHERE username LIKE ?1 AND id != ?2 LIMIT ?3")?;
        let pattern = format!("{}%", query);

        let users = stmt.query_map(params![pattern, my_id.to_string(), limit], |row| {
            let id: String = row.get(0)?;
            Ok((Uuid::parse_str(&id).unwrap(), row.get(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(users)
    }

    // SESSIONI
    pub fn insert_session(&self, username: &str, ttl_secs: i64) -> AppResult<Uuid> {
        let token = Uuid::new_v4();
        // Usiamo now_unix() aggiornato ai millisecondi
        let now = now_unix();
        // Convertiamo ttl in ms per coerenza, o lo trattiamo come secondi aggiunti ai ms
        let expires = now + (ttl_secs * 1000); 
        
        self.conn.execute("INSERT INTO sessions(token, user_id, created_at, expires_at) VALUES (?1, (SELECT id FROM users WHERE username = ?2), ?3, ?4)",
                          params![token.to_string(), username, now, expires])?;
        Ok(token)
    }

    pub fn validate_session(&self, token: &Uuid) -> AppResult<Option<Uuid>> {
        let now = now_unix();
        let row = self.conn.query_row("SELECT user_id FROM sessions WHERE token = ?1 AND expires_at > ?2", params![token.to_string(), now], |r| {
            let s: String = r.get(0)?;
            Ok(Uuid::parse_str(&s).unwrap())
        }).optional()?;
        Ok(row)
    }

    // CHAT PRIVATE
    pub fn create_private_chat(&self, user1_id: &Uuid, user2_id: &Uuid) -> AppResult<Uuid> {
        let (u1, u2) = if user1_id < user2_id { (user1_id, user2_id) } else { (user2_id, user1_id) };
        let chat_id = Uuid::new_v4();
        match self.conn.execute("INSERT INTO private_chats(id, user1_id, user2_id, created_at) VALUES (?1, ?2, ?3, ?4)", params![chat_id.to_string(), u1.to_string(), u2.to_string(), now_unix()]) {
            Ok(_) => Ok(chat_id),
            Err(e) if is_unique_violation(&e) => {
                let ex = self.conn.query_row("SELECT id FROM private_chats WHERE user1_id = ?1 AND user2_id = ?2", params![u1.to_string(), u2.to_string()], |r| {
                    let s: String = r.get(0)?;
                    Ok(Uuid::parse_str(&s).unwrap())
                })?;
                Ok(ex)
            }
            Err(e) => Err(AppError::Db(e)),
        }
    }

    pub fn get_user_private_chats(&self, user_id: &Uuid) -> AppResult<Vec<(Uuid, Uuid, String)>> {
        let mut stmt = self.conn.prepare(r#"
            SELECT pc.id, CASE WHEN pc.user1_id = ?1 THEN pc.user2_id ELSE pc.user1_id END, u.username
            FROM private_chats pc JOIN users u ON u.id = CASE WHEN pc.user1_id = ?1 THEN pc.user2_id ELSE pc.user1_id END
            WHERE pc.user1_id = ?1 OR pc.user2_id = ?1 ORDER BY pc.created_at DESC
        "#)?;
        let chats = stmt.query_map([user_id.to_string()], |row| {
            let cid: String = row.get(0)?;
            let uid: String = row.get(1)?;
            Ok((Uuid::parse_str(&cid).unwrap(), Uuid::parse_str(&uid).unwrap(), row.get(2)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(chats)
    }

    pub fn is_user_in_private_chat(&self, user_id: &Uuid, chat_id: &Uuid) -> AppResult<bool> {
        let c: i32 = self.conn.query_row("SELECT COUNT(*) FROM private_chats WHERE id = ?1 AND (user1_id = ?2 OR user2_id = ?2)", params![chat_id.to_string(), user_id.to_string()], |r| r.get(0))?;
        Ok(c > 0)
    }

    pub fn get_private_chat_members(&self, chat_id: &Uuid) -> AppResult<Option<(Uuid, Uuid)>> {
        let row = self.conn.query_row(
            "SELECT user1_id, user2_id FROM private_chats WHERE id = ?1",
            [chat_id.to_string()],
            |r| {
                let u1: String = r.get(0)?;
                let u2: String = r.get(1)?;
                Ok((Uuid::parse_str(&u1).unwrap(), Uuid::parse_str(&u2).unwrap()))
            }
        ).optional()?;
        Ok(row)
    }

    // GRUPPI
    pub fn create_group(&self, name: &str, creator_id: &Uuid) -> AppResult<Uuid> {
        let gid = Uuid::new_v4();
        self.conn.execute("INSERT INTO groups(id, name, creator_id, created_at) VALUES (?1, ?2, ?3, ?4)", params![gid.to_string(), name, creator_id.to_string(), now_unix()])?;
        self.add_group_member(&gid, creator_id)?;
        Ok(gid)
    }

    pub fn add_group_member(&self, group_id: &Uuid, user_id: &Uuid) -> AppResult<()> {
        self.conn.execute("INSERT OR IGNORE INTO group_members(group_id, user_id, joined_at) VALUES (?1, ?2, ?3)", params![group_id.to_string(), user_id.to_string(), now_unix()])?;
        Ok(())
    }

    pub fn is_user_in_group(&self, user_id: &Uuid, group_id: &Uuid) -> AppResult<bool> {
        let c: i32 = self.conn.query_row("SELECT COUNT(*) FROM group_members WHERE group_id = ?1 AND user_id = ?2", params![group_id.to_string(), user_id.to_string()], |r| r.get(0))?;
        Ok(c > 0)
    }

    pub fn get_user_groups(&self, user_id: &Uuid) -> AppResult<Vec<(Uuid, String)>> {
        let mut stmt = self.conn.prepare("SELECT g.id, g.name FROM groups g JOIN group_members gm ON g.id = gm.group_id WHERE gm.user_id = ?1 ORDER BY g.created_at DESC")?;
        let groups = stmt.query_map([user_id.to_string()], |row| {
            let gid: String = row.get(0)?;
            Ok((Uuid::parse_str(&gid).unwrap(), row.get(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(groups)
    }

    pub fn get_group_members(&self, group_id: &Uuid) -> AppResult<Vec<(Uuid, String)>> {
        let mut stmt = self.conn.prepare("SELECT u.id, u.username FROM users u JOIN group_members gm ON u.id = gm.user_id WHERE gm.group_id = ?1")?;
        let members = stmt.query_map([group_id.to_string()], |row| {
            let uid: String = row.get(0)?;
            Ok((Uuid::parse_str(&uid).unwrap(), row.get(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(members)
    }

    // MESSAGGI
    pub fn insert_message(&self, sender_id: &Uuid, content: &str, private_chat_id: Option<&Uuid>, group_id: Option<&Uuid>) -> AppResult<Uuid> {
        let mid = Uuid::new_v4();
        let pid = private_chat_id.map(|u| u.to_string());
        let gid = group_id.map(|u| u.to_string());
        self.conn.execute("INSERT INTO messages(id, sender_id, private_chat_id, group_id, content, sent_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                          params![mid.to_string(), sender_id.to_string(), pid, gid, content, now_unix()])?;
        Ok(mid)
    }

    pub fn get_private_chat_messages(&self, chat_id: &Uuid, limit: u32) -> AppResult<Vec<(Uuid, Uuid, String, String, i64)>> {
        let mut stmt = self.conn.prepare("SELECT m.id, m.sender_id, u.username, m.content, m.sent_at FROM messages m JOIN users u ON m.sender_id = u.id WHERE m.private_chat_id = ?1 ORDER BY m.sent_at DESC LIMIT ?2")?;
        let msgs = stmt.query_map(params![chat_id.to_string(), limit], |row| {
            let mid: String = row.get(0)?;
            let sid: String = row.get(1)?;
            Ok((Uuid::parse_str(&mid).unwrap(), Uuid::parse_str(&sid).unwrap(), row.get(2)?, row.get(3)?, row.get(4)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(msgs)
    }

    pub fn get_group_messages(&self, group_id: &Uuid, limit: u32) -> AppResult<Vec<(Uuid, Uuid, String, String, i64)>> {
        let mut stmt = self.conn.prepare("SELECT m.id, m.sender_id, u.username, m.content, m.sent_at FROM messages m JOIN users u ON m.sender_id = u.id WHERE m.group_id = ?1 ORDER BY m.sent_at DESC LIMIT ?2")?;
        let msgs = stmt.query_map(params![group_id.to_string(), limit], |row| {
            let mid: String = row.get(0)?;
            let sid: String = row.get(1)?;
            Ok((Uuid::parse_str(&mid).unwrap(), Uuid::parse_str(&sid).unwrap(), row.get(2)?, row.get(3)?, row.get(4)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(msgs)
    }
}

fn is_unique_violation(e: &rusqlite::Error) -> bool {
    matches!(e, rusqlite::Error::SqliteFailure(err, _) if err.code == rusqlite::ErrorCode::ConstraintViolation)
}


fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}


