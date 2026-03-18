# 🦀 Ruggine – Chat Testuale Client/Server in Rust

Ruggine è un’applicazione **client/server** realizzata in **Rust** per gestire chat testuali multi–utente.  
Il progetto implementa un’architettura con comunicazione **TCP** e payload **JSON**, sfruttando **Tokio** per la concorrenza e **egui/eframe** per l’interfaccia grafica lato client.

---

## ⚙️ Architettura generale

- **Protocollo**: TCP con codec a lunghezza (`tokio_util::codec::LengthDelimitedCodec`).
- **Messaggi**: scambio di `ClientMsg` ↔ `ServerMsg` serializzati in JSON.
- **Autenticazione**: password cifrate con **Argon2 + salt**.
- **Database**: SQLite locale (`rusqlite`).
- **Piattaforme testate**: Windows e Linux.

---
Dopo il gitclone, fare cargo run da terminale per buildare il progetto
- Per runnare:
 - Dal primo terminale cargo run -p client
 - Dal secondo terminale cargo run -p server




