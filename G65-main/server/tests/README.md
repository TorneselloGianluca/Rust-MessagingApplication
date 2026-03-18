# End-to-End Test Suite per il Server di Messaggistica

Questa directory contiene la suite di **test end-to-end (E2E)** sviluppata per verificare il comportamento reale del server di messaggistica.  
I test avviano un’istanza effettiva del server, si connettono tramite TCP (con codec length-delimited) e scambiano messaggi utilizzando il protocollo definito dall’applicazione.

L’obiettivo è controllare non solo che le singole funzioni del server siano corrette, ma che l’intero sistema — rete, gestione degli utenti, gruppi, messaggistica e persistenza — funzioni in condizioni realistiche.

La suite è composta da:

- **`e2e_functionals.rs`** — Test funzionali del flusso utente principale
- **`e2e_stress_tests.rs`** — Test di carico, concorrenza e comportamento sotto stress

---

# Obiettivi dei test

La suite verifica che:

- Il **protocollo client-server** sia implementato correttamente end-to-end.
- Le principali operazioni dell’applicazione (registrazione, login, gruppi, messaggi) funzionino in scenari reali.
- Il server mantenga un comportamento coerente anche in presenza di:
  - più utenti attivi,
  - elevato numero di messaggi,
  - ritardi,
  - riconnessioni,
  - condizioni limite.
- Non emergano errori strutturali come deadlock, race condition o blocchi dell’esecuzione.

Questi test permettono quindi di valutare **correttezza, consistenza e robustezza** dell’intero sistema.

---

# Contenuto dei file

## 1. `e2e_functionals.rs` — Test funzionali completi

Questi test riproducono i flussi tipici dell’applicazione e verificano la correttezza logica del server.

### Funzionalità testate

- **Registrazione di nuovi utenti**  
  Verifica che il server gestisca correttamente la creazione degli account.

- **Login e gestione della sessione**  
  Controlla la risposta del server e il riconoscimento dell’identità utente.

- **Messaggi privati tra due utenti**  
  I test verificano:
  - l’invio,
  - la ricezione,
  - l’ordine dei messaggi,
  - la correttezza dei metadati.

- **Creazione di gruppi e aggiunta dei membri**  
  Il server deve creare un nuovo gruppo e riconoscere correttamente i membri invitati.

- **Invio e ricezione di messaggi nei gruppi**  
  Viene verificata:
  - la consegna a tutti i membri del gruppo,
  - l’assenza di duplicati,
  - la coerenza dell’ordine.

- **Verifica dei metadati dei messaggi (`MessageInfo`)**  
  Inclusi:
  - autore,
  - timestamp,
  - ID del gruppo o del destinatario,
  - contenuto.

- **Gestione degli errori previsti**  
  Come:
  - utente non loggato,
  - gruppo inesistente,
  - messaggi a utenti assenti,
  - tentativi di accesso non autorizzati.

Questi test dimostrano che la logica del server sia implementata correttamente e che il protocollo risponda in modo coerente e prevedibile.

---

## 2. `e2e_stress_tests.rs` — Test di stress e concorrenza

Questi test valutano la stabilità del server sotto condizioni impegnative.

### Situazioni simulate

- **Connessioni simultanee da parte di più utenti**  
  Serve a valutare se il server è in grado di accettare rapidamente connessioni multiple senza degradare.

- **Invio concorrente di messaggi privati e di gruppo**  
  Diversi task asincroni inviano messaggi in parallelo.

- **Burst di messaggi ad alta intensità**  
  L’obiettivo è individuare colli di bottiglia nel sistema o rallentamenti inattesi.

- **Timeout controllati sulle risposte del server**  
  I test falliscono se il server non risponde entro una finestra temporale definita, verificando la reattività del sistema.

- **Creazione e gestione ripetuta di gruppi**  
  Verifica la robustezza dell’algoritmo di gestione dei gruppi e la consistenza dello stato interno.

### Cosa evidenziano

- **Scalabilità del server**  
  Il server mantiene prestazioni stabili anche quando il traffico aumenta.

- **Robustezza dell’implementazione asincrona (Tokio)**  
  Se non si verificano deadlock o ritardi anomali, la gestione della concorrenza è corretta.

- **Resistenza a condizioni non ideali**  
  Come molti messaggi in coda, ritardi, gruppi creati e cancellati rapidamente, ecc.

---

### Come eseguire i test

```text
cargo test -p server --test e2e_functionals -- --nocapture
```

```text
cargo test -p server --test e2e_stress_tests -- --nocapture
```

## 3. `e2e_private_chat.rs` - Test di stress su chat privata

Questo modulo implementa un test **End-to-End (E2E) di carico** per il server di chat. A differenza dei test unitari, questo test avvia un'istanza reale del server in un processo separato e simula client TCP reali che interagiscono con esso.

### Obiettivi del Test

1.  **Stress Test:** Verificare la stabilità del server sotto carico crescente (fino a 150 client concorrenti).
2.  **Concorrenza:** Assicurare che il server gestisca correttamente connessioni simultanee e il broadcast dei messaggi.
3.  **Persistenza e Affidabilità:** Misurare l'efficienza del sistema confrontando i messaggi inviati con quelli effettivamente processati.

---

### Architettura del Test

#### 1. Il Client Simulato (`TestClient`)
La struct `TestClient` agisce come un utente reale. Ogni istanza:
* Apre una connessione TCP verso `127.0.0.1`.
* Utilizza `Framed<TcpStream>` con `LengthDelimitedCodec` per la gestione dei pacchetti.
* Esegue automaticamente il flusso di autenticazione:
    1.  **Register:** Registrazione nuovo utente.
    2.  **Login:** Accesso e ottenimento del `session_token`.
    3.  **Listen:** Sottoscrizione al flusso di eventi in entrata.

#### 2. Gestione del Processo Server
Il test è autonomo e non richiede un server già attivo:
* **Spawn:** Compila e avvia il binario `server` come processo figlio (`Child`).
* **Porta Random:** Seleziona una porta casuale (9000-10000) per evitare conflitti tra test paralleli.
* **DB Isolato:** Crea un database SQLite temporaneo per ogni scenario, che viene eliminato alla fine del test.

---

### Flusso di Esecuzione (`run_scenario`)

La funzione `run_scenario` orchestra un singolo livello di test seguendo questi passaggi:

1.  **Boot:** Avvia il server e attende che la porta TCP sia raggiungibile.
2.  **Popolazione:** Crea `N` client e li connette al server.
3.  **Creazione Anello di Chat:**
    * Ogni utente `i` avvia una chat privata con l'utente successivo `(i + 1)`.
    * Questo garantisce che ogni client sia sia mittente che destinatario.
4.  **Sincronizzazione (Barrier):**
    * Viene usata una `tokio::sync::Barrier` per assicurare che **tutti** i client inizino a inviare messaggi nello stesso millisecondo, massimizzando il picco di carico.
5.  **Load Loop:**
    * Per una durata definita (es. 60s), ogni client invia un messaggio e attende l'ack (`PrivateMessageSent`).
6.  **Report & Cleanup:**
    * I risultati vengono calcolati e scritti su file.
    * Il server viene terminato (`kill`) e i file temporanei rimossi.

---

### Scenari di Test

Il test esegue sequenzialmente 5 scenari di difficoltà crescente. Ogni scenario dura **60 secondi** con un rate di **1 messaggio/secondo** per utente.

| Scenario | Utenti (Thread) | Messaggi Totali Attesi (approx) | Descrizione |
| :--- | :--- | :--- | :--- |
| **Test 1** | 10 | 600 | Warm-up base. |
| **Test 2** | 25 | 1.500 | Carico leggero. |
| **Test 3** | 50 | 3.000 | Carico medio. |
| **Test 4** | 100 | 6.000 | Carico alto. |
| **Test 5** | 150 | 9.000 | Stress test massimo. |

> **Nota:** Il test è configurato con `worker_threads = 16` per garantire che il framework di test non diventi il collo di bottiglia durante la simulazione di 150 client.

---

### Come eseguire il test

```text
cargo test -p server --test e2e_private_chat
```

### Logging dei Risultati

I risultati non vengono mostrati solo in console, ma vengono appesi al file:
`server/tests/results/e2e_private_chat_res.txt`

Il formato del log permette di tracciare le performance nel tempo:

```text
__________________________________________
TEST 5:
- Info: 150 Users, 60 sec duration, 1000 ms interval
- Data: 27/11/2023 10:00
- Risultato: 8950 / 9000 messaggi (Efficienza 99.44%)
__________________________________________
```

## 3. `e2e_private_chat_monitoring.rs` - Ottimizzazione Stress Test & Database Tuning

Durante l'esecuzione degli Stress Test (in particolare lo scenario con **250 utenti concorrenti** aggiunto in fase di monitoring), 
sono emerse criticità legate alla concorrenza sul database SQLite e alla gestione delle risorse di sistema. 
Questa sezione documenta i problemi riscontrati e le soluzioni tecniche implementate per raggiungere il **100% di efficienza**.

### Come eseguire il test

```text
cargo test -p server --test e2e_private_chat_monitoring
```

### 1. Analisi dei Problemi

#### Database Locked & Bassa Efficienza
Nel **Test 6 (250 utenti)**, l'efficienza iniziale è crollata al **64.61%**, con un elevato utilizzo della CPU e frequenti errori di lock.

```text
TEST 6:
- Info: 250 Users, 60 sec duration
- Performance Server: Avg CPU: 84.10%
- Risultato: 9691 / 15000 messaggi (Efficienza 64.61%)

[SERVER ERROR]: db error: database is locked
[SERVER ERROR]: db error: database is locked
```

**Causa**: SQLite è un database basato su file che, di default, permette un solo scrittore alla volta. Quando 250 utenti tentano di inviare messaggi simultaneamente, si crea una coda di scrittura ingestibile. Se un thread attende il lock oltre il limite, SQLite va in timeout.

#### Timestamp & Ordinamento Messaggi

**Causa**: La funzione originale usava as_secs() (secondi). Sotto stress test, centinaia di messaggi venivano salvati con lo stesso identico timestamp (es. 1700001234), rendendo l'ordinamento cronologico (ORDER BY sent_at DESC) imprevedibile.

### 2. Soluzioni: Database Tuning (`storage.rs`)

Abbiamo riconfigurato `SqliteStorage::new` con tre direttive PRAGMA fondamentali per l'alta concorrenza e migliorato la precisione temporale.

#### Configurazione SQLite
1.  **`journal_mode = WAL` (Write-Ahead Logging)**
    * Invece di bloccare l'intero database, le scritture avvengono in un file separato (`.wal`).
    * 
    * **Risultato:** Lettori e scrittori possono operare contemporaneamente senza bloccarsi.

2.  **`busy_timeout = 5000`**
    * Imposta un tempo di attesa di 5 secondi. Se il DB è occupato, il server aspetta invece di restituire errore immediato.

3.  **`synchronous = FULL`**
    * Inizialmente impostato su `NORMAL` per velocità, causava errori di login ("Credenziali Errate") perché la `SELECT` avveniva prima che la `INSERT` della registrazione fosse fisicamente scritta su disco (Race Condition).
    * **Risultato:** Riportandolo a `FULL`, garantiamo la persistenza immediata del dato, eliminando gli errori di login.

#### Precisione Timestamp
Abbiamo aggiornato la funzione `now_unix` per usare i millisecondi:

```rust
fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Usa as_millis() invece di as_secs()
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}
```

**Risultato**: I conflitti di ordinamento sono statisticamente eliminati.

### 3. Ottimizzazione Codice di Stress Test

Per evitare che il test stesso diventasse il collo di bottiglia (falsi negativi), abbiamo applicato diverse ottimizzazioni architetturali al client di test:

#### 🚦 Traffic Shaping (Ramp-up)
* **Problema:** Lanciare 250 connessioni nello stesso millisecondo causava un "Thundering Herd", saturando il server istantaneamente.
* **Soluzione:** Inserita una micro-pausa (`5ms`) tra la connessione di un client e l'altro nel ciclo di inizializzazione.

#### 🔄 Smart Retry (Resilienza)
* **Logica:** Con SQLite in modalità WAL sincrona, esiste una minima latenza fisica di scrittura.
* **Soluzione:** Se il login fallisce ("Credenziali errate" o "Database locked"), il client non crasha ma attende **200ms** e riprova. Inoltre, in caso di porta TCP satura, riprova la connessione fino a 10 secondi.

#### 📉 Monitoring a Bassa Frequenza
* **Problema:** La libreria `sysinfo` consumava troppa CPU aggiornando ogni secondo, rubando risorse al server.
* **Soluzione:** Ridotta la frequenza di campionamento a **3 secondi**.

#### 🧵 Tokio Worker Threads
* **Configurazione:** `#[tokio::test(flavor = "multi_thread", worker_threads = 32)]`
* **Vantaggio:** Aumentare i thread a 32 permette al runtime asincrono di gestire meglio i 250 task client + server + monitoring, riducendo il context switching.