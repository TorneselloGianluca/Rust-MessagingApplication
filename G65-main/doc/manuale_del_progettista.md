# MANUALE DEL PROGETTISTA

## INDICE

- CLIENT
  - MODULO `lib.rs`
    - Enum `ClientMsg`
      - Autenticazione
      - Ricerca utenti
      - Chat privata
      - Gestione gruppi
    - Enum `ServerMsg`
      - Risposte alle richieste del client
      - Notifiche push
    - Struct `UserInfo`
    - Struct `PrivateChatInfo`
    - Struct `GroupInfo`
    - Struct `MessageInfo`
  - MODULO `app.rs`
    - Enum `View`
    - Struct `App`
    - Enum `AppResult`
    - Metodi principali di `App`
    - Implementazione `eframe::App` - Metodo `update`
      - Disegno della UI
        - Pannello superiore (`TopBottomPanel::top`)
        - Pannello centrale (`CentralPanel`)
      - Gestione degli eventi asincroni
        - Esempi di gestione dei risultati
      - Aggiornamento periodico dell’interfaccia
  - MODULO `net.rs`
    - Funzioni principali
      - [Funzione: send_and_receive]
      - [Funzione: listen_background]
    - Autenticazione
      - [Funzione: login]
      - [Funzione: register]
    - Ricerca utenti
      - [Funzione: search_users]
    - Chat private
      - [Funzione: start_private_chat]
      - [Funzione: get_private_chats]
      - [Funzione: get_private_messages]
      - [Funzione: send_private_message]
    - Gestione gruppi
      - [Funzione: create_group]
      - [Funzione: add_group_member]
      - [Funzione: get_groups]
      - [Funzione: get_group_members]
      - [Funzione: get_group_messages]
      - [Funzione: send_group_message]
    - Flusso di comunicazione `net.rs` → `app.rs`
      - Operazioni
        - 1. Registrazione utente
        - 2. Login utente
        - 3. Ricerca utente e creazione chat privata
        - 4. Scambio di messaggi in chat privata
        - 5. Creazione di un gruppo
        - 6. Scambio di messaggi in chat di gruppo
        - 7. Logout utente
- SERVER
  - MODULO `storage.rs`
    - Struct `SqliteStorage`
      - [Funzione: new]
      - [Funzione: init]
    - Gestione utenti
      - [Funzione: insert_user]
      - [Funzione: get_pwd_hash]
      - [Funzione: get_user_id]
      - [Funzione: get_username]
      - [Funzione: search_users]
    - Gestione sessioni
      - [Funzione: insert_session]
      - [Funzione: validate_session]
    - Gestione chat private
      - [Funzione: create_private_chat]
      - [Funzione: get_user_private_chats]
      - [Funzione: is_user_in_private_chat]
      - [Funzione: get_private_chat_members]
    - Gestione gruppi
      - [Funzione: create_group]
      - [Funzione: add_group_member]
      - [Funzione: is_user_in_group]
      - [Funzione: get_user_groups]
      - [Funzione: get_group_members]
    - Gestione messaggi
      - [Funzione: insert_message]
      - [Funzione: get_private_chat_messages]
      - [Funzione: get_group_messages]
    - Funzioni di supporto
      - [Funzione: is_unique_violation]
      - [Funzione: now_unix]
  - MODULO `handlers.rs`
    - Autenticazione e ricerca utenti
      - [Funzione: handle_register]
      - [Funzione: handle_login]
      - [Funzione: handle_search_users]
    - Chat private
      - [Funzione: handle_start_private_chat]
      - [Funzione: handle_get_private_chats]
      - [Funzione: handle_get_private_chat_messages]
      - [Funzione: handle_send_private_message]
    - Gestione gruppi
      - [Funzione: handle_create_group]
      - [Funzione: handle_add_group_member]
      - [Funzione: handle_get_groups]
      - [Funzione: handle_get_group_members]
      - [Funzione: handle_get_group_messages]
      - [Funzione: handle_send_group_message]
    - Note generali
  - MODULO `auth.rs`
    - Librerie utilizzate
    - Funzioni principali
      - [Funzione: hash_password]
      - [Funzione: verify_password]
    - Note generali
  - MODULO `errors.rs`
    - Librerie utilizzate
    - Enum `AppError`
    - Tipo `AppResult<T>`
  - MODULO `net.rs`
    - Librerie utilizzate
    - Funzioni principali
      - [Funzione: serve_connection]
      - [Funzione: handle_client_msg]
      - [Funzione: map_error]
      - [Funzione: send]
    - Note sul funzionamento
    - Messaggi supportati
  - MODULO `main.rs`
    - Moduli interni importati
    - Costanti e tipi principali
    - Funzione principale asincrona
      - Avvio del monitoring
      - Inizializzazione del database
      - Creazione del listener TCP
      - Creazione della PeerMap condivisa
      - Loop principale delle connessioni
    - Riassunto
    - Flusso generico di comunicazione tra client e server
    - Flussi specifici (Registrazione, Login, Logout, Ricerca utente, Messaggi privati, Messaggi di gruppo)
    - Nota sull’utilizzo della `PeerMap`

## CLIENT

### MODULO `lib.rs`

Questo modulo contiene tutte le strutture dati condivise tra client e server.
Rappresenta il linguaggio comune con cui i due componenti comunicano tramite messaggi serializzati (grazie a serde).

Include:

- ClientMsg → messaggi inviati dal client → server  
- ServerMsg → messaggi inviati dal server → client  
- Strutture comuni (UserInfo, PrivateChatInfo, GroupInfo, MessageInfo) che descrivono utenti, gruppi, chat private e messaggi.

L'intero modulo deriva i trait `Serialize` e `Deserialize` per garantire la conversione automatica da/verso JSON o qualsiasi altro formato supportato.

---

#### Enum `ClientMsg`

`ClientMsg` rappresenta tutte le possibili richieste che il client può inviare al server.  
Ogni variante della enum è un messaggio strutturato, con i parametri necessari per svolgere operazioni di autenticazione, ricerca utenti, gestione delle chat private e dei gruppi.

Le operazioni principali sono:

---

##### **Autenticazione**

- `Register { username, password }` → Richiesta di registrazione di un nuovo utente.
- `Login { username, password }` → Accesso al sistema.
- `Listen { token }` → Il client si mette in ascolto delle notifiche push del server.

---

##### **Ricerca utenti**

- `SearchUsers { token, query }` → Permette di cercare utenti per nome.

---

##### **Chat privata**

- `StartPrivateChat { token, other_username }` → Avvia (o recupera) una chat privata.
- `GetPrivateChats { token }` → Ottiene la lista delle chat private dell’utente.
- `GetPrivateChatMessages { token, chat_id, limit }` → Recupera i messaggi di una chat privata.
- `SendPrivateMessage { token, chat_id, content }` → Invia un messaggio privato.

---

##### **Gestione gruppi**

- `CreateGroup { token, name }` → Crea un nuovo gruppo.
- `AddGroupMember { token, group_id, username }` → Aggiunge un utente a un gruppo.
- `GetGroups { token }` → Ottiene la lista dei gruppi dell’utente.
- `GetGroupMembers { token, group_id }` → Ottiene la lista dei membri del gruppo.
- `GetGroupMessages { token, group_id, limit }` → Recupera i messaggi di un gruppo.
- `SendGroupMessage { token, group_id, content }` → Invia un messaggio all’interno di un gruppo.

---

#### Enum `ServerMsg`

`ServerMsg` rappresenta tutte le risposte e notifiche che il server invia al client.
Comprende risposte dirette, notifiche push e messaggi di sistema.

---

##### **Risposte alle richieste del client**

- `Registered { user_id }` → Conferma avvenuta registrazione.
- `LoginOk { session_token, user_id, username }` → Accesso riuscito.
- `UsersFound { users }` → Risultati della ricerca utenti.
- `PrivateChatStarted { chat_id }` → Una chat privata è stata avviata.
- `PrivateChats { chats }` → Lista delle chat private dell’utente.
- `PrivateChatMessages { messages }` → Cronologia della chat privata.
- `PrivateMessageSent { message_id }` → Conferma di invio messaggio privato.
- `GroupCreated { group_id }` → Conferma creazione gruppo.
- `GroupMemberAdded` → Conferma aggiunta utente al gruppo.
- `Groups { groups }` → Lista dei gruppi dell’utente.
- `GroupMembers { members }` → Lista dei membri del gruppo.
- `GroupMessages { messages }` → Cronologia dei messaggi del gruppo.
- `GroupMessageSent { message_id }` → Conferma invio messaggio di gruppo.
- `Error { message }` → Segnalazione di errore.

---

##### **Notifiche push**

- `PushNewMessage { message, chat_id, group_id }` → Nuovo messaggio ricevuto in chat privata o gruppo.
- `PushGroupUpdated` → Aggiornamenti relativi ai gruppi.
- `PushPrivateChatListUpdated` → Aggiornamento della lista delle chat private.

---

#### Struct `UserInfo`

Rappresenta le informazioni di un utente.

**Campi:**
- `user_id`: Uuid  
- `username`: String  

---

#### Struct `PrivateChatInfo`

Contiene le informazioni necessarie a rappresentare una chat privata.

**Campi:**
- `chat_id`: Uuid  
- `other_user_id`: Uuid  
- `other_username`: String  

---

#### Struct `GroupInfo`

Rappresenta un gruppo.

**Campi:**
- `group_id`: Uuid  
- `name`: String  

---

#### Struct `MessageInfo`

Rappresenta un singolo messaggio scambiato nel sistema.

**Campi:**
- `message_id`: Uuid  
- `sender_id`: Uuid  
- `sender_username`: String  
- `content`: String  
- `sent_at`: i64  

### MODULO `app.rs`

Il file `app.rs` gestisce l’interfaccia grafica principale dell’applicazione di messaggistica tramite la libreria **egui** ed `eframe`.  
Si occupa di:

- Mantenere lo stato dell’applicazione (login, registrazione, chat, gruppi, messaggi);
- Coordinare la navigazione tra le varie schermate (`View`);
- Gestire la comunicazione asincrona con il server tramite canali (`mpsc`) e runtime **Tokio**;
- Aggiornare l’interfaccia e reagire ai messaggi ricevuti dal backend.

---

#### Enum `View`

Rappresenta le schermate o viste principali dell’app:

- `Home` → Schermata iniziale dell’app.
- `Login` → Form di login per utenti registrati.
- `Register` → Form di registrazione per nuovi utenti.
- `ChatList` → Lista di chat e gruppi disponibili.
- `PrivateChat(usize)` → Chat privata selezionata, indicizzata nel vettore `private_chats`.
- `GroupChat(usize)` → Chat di gruppo selezionata, indicizzata nel vettore `groups`.

L’implementazione di `Default` imposta la vista iniziale su `Home`.

---

#### Struct `App`

`App` rappresenta l’intera applicazione e contiene **lo stato della UI, dati dell’utente, messaggi, chat, gruppi e canali di comunicazione**.

##### Dati di login/registrazione

- `server_addr: String` → indirizzo del server (es. `"127.0.0.1:7878"`).
- `login_username` / `login_password` → credenziali temporanee per login.
- `reg_username` / `reg_password` → credenziali temporanee per registrazione.
- `session_token: Option<Uuid>` → token di sessione ottenuto dal server.
- `user_id: Option<Uuid>` → identificativo utente.
- `username: Option<String>` → username dell’utente loggato.

##### Ricerca utenti

- `search_query: String` → stringa digitata dall’utente per cercare altri utenti.
- `search_results: Vec<UserInfo>` → risultati della ricerca.
- `last_search_completed: bool` → flag che indica il completamento della ricerca.
- `search_last_edit: Option<Instant>` → timestamp dell’ultimo aggiornamento della ricerca.

##### Chat e messaggi

- `private_chats: Vec<PrivateChatInfo>` → elenco delle chat private dell’utente.
- `groups: Vec<GroupInfo>` → elenco dei gruppi di cui fa parte l’utente.
- `chats_loaded: bool` → indica se le chat sono state caricate.
- `current_messages: Vec<MessageInfo>` → messaggi attualmente visualizzati.
- `current_group_members: Vec<UserInfo>` → partecipanti del gruppo corrente.
- `message_input: String` → testo inserito dall’utente da inviare.

##### Gestione gruppi

- `new_group_name: String` → nome del gruppo in fase di creazione.
- `add_member_username: String` → username del membro da aggiungere.
- `show_group_info: bool` → flag per mostrare informazioni aggiuntive del gruppo.

##### Stato UI e caricamento

- `status: String` → messaggi di stato o errore.
- `is_loading: bool` → indica se l’app sta effettuando un’operazione in background.
- `rt: Option<Runtime>` → runtime Tokio per eseguire operazioni asincrone.
- `rx_result: Option<mpsc::Receiver<AppResult>>` → ricezione dei risultati asincroni.
- `tx_result: Option<mpsc::Sender<AppResult>>` → invio dei comandi/risultati al main loop.

---

#### Enum `AppResult`

Rappresenta i risultati asincroni ricevuti dal backend:

- `LoginSuccess { token, user_id, username }`
- `RegisterSuccess`
- `SearchResults { users }`
- `PrivateChatsLoaded { chats }`
- `GroupsLoaded { groups }`
- `GroupMembersLoaded { members }`
- `MessagesLoaded { messages }`
- `PrivateChatStarted { chat_id }`
- `GroupCreated { group_id }`
- `MessageSent`
- `MemberAdded`
- `Error { message }`
- `PushGroupListUpdated`
- `PushPrivateChatListUpdated`
- `PushNewMessage { message, chat_id, group_id }`

---

#### Metodi principali di `App`

##### Navigazione e stato

- `nav(&mut self, v: View)` → cambia la vista corrente e resetta lo stato.
- `reset_status(&mut self)` → cancella messaggi di stato e disattiva il caricamento.
- `is_logged_in(&self) -> bool` → verifica se l’utente è loggato.
- `logout(&mut self)` → resetta l’app e torna alla vista `Home`.
- `get_tx(&mut self) -> mpsc::Sender<AppResult>` → restituisce il canale di invio per i risultati asincroni.

##### Caricamento dati dal server

- `load_private_chats(&mut self, ctx: &egui::Context, token: Uuid, show_loading: bool)`
- `load_groups(&mut self, ctx: &egui::Context, token: Uuid, show_loading: bool)`
- `load_private_messages(&mut self, ctx: &egui::Context, token: Uuid, chat_id: Uuid, show_loading: bool)`
- `load_group_messages(&mut self, ctx: &egui::Context, token: Uuid, group_id: Uuid, show_loading: bool)`
- `load_group_members(&mut self, ctx: &egui::Context, token: Uuid, group_id: Uuid)`

Questi metodi eseguono richieste asincrone al server e aggiornano lo stato dell’app tramite `AppResult`.

---

#### Implementazione `eframe::App` - Metodo `update`

Il metodo `update` definisce il ciclo principale dell’interfaccia grafica e gestisce la logica dell’applicazione in tempo reale.  

Si occupa di:

1. **Inizializzazione del runtime Tokio e canali di comunicazione**  
   - Se il runtime non esiste, viene creato per eseguire operazioni asincrone (fetching dati dal server).  
   - Vengono creati i canali `mpsc::Sender` e `Receiver` per ricevere i risultati delle operazioni di rete.

2. **Impostazione dell’indirizzo del server**  
   - Se `server_addr` è vuoto, viene impostato un valore di default `"127.0.0.1:7878"`.

---

##### Disegno della UI

###### Pannello superiore (`TopBottomPanel::top`)
- Contiene il titolo dell’app e lo stato di login dell’utente.
- Se l’utente è loggato, mostra:
  - Bottone **Logout** per terminare la sessione.
  - Label con il nome dell’utente loggato.

###### Pannello centrale (`CentralPanel`)
- Gestisce le viste principali a seconda di `self.view`:
  - `Home` → schermata iniziale.
  - `Login` → form di login.
  - `Register` → form di registrazione.
  - `ChatList`, `PrivateChat(idx)`, `GroupChat(idx)` → delega la visualizzazione ai moduli `views`.

- Mostra anche lo **stato di caricamento**:
  - Se `is_loading` → spinner + testo dello stato.
  - Altrimenti, visualizza messaggi di errore o informazioni brevi.

---

##### Gestione degli eventi asincroni

- Raccoglie i messaggi ricevuti dal canale `rx_result` in un vettore `events`.
- Itera su ciascun `AppResult` e aggiorna lo stato dell’app:

###### Esempi di gestione dei risultati

- **LoginSuccess**
  - Imposta `session_token`, `user_id` e `username`.
  - Naviga automaticamente alla vista `ChatList`.
  - Avvia il listener di messaggi in background tramite Tokio.

- **PushNewMessage**
  - Controlla se il messaggio appartiene alla chat corrente.
  - Aggiunge il messaggio a `current_messages` solo se non esiste già.

- **GroupsLoaded / PrivateChatsLoaded**
  - Aggiorna le liste di gruppi o chat private.
  - Eventualmente lancia il caricamento automatico dei membri del gruppo.

- **Error**
  - Aggiorna lo stato con il messaggio di errore.
  - Disabilita `is_loading`.

- **RegisterSuccess / MemberAdded / MessageSent / GroupCreated**
  - Aggiornano lo stato, puliscono campi temporanei e ricaricano dati se necessario.

---

##### Aggiornamento periodico dell’interfaccia

- Alla fine di `update` viene richiesto un **repaint ogni 200ms**:
  ctx.request_repaint_after(Duration::from_millis(200));

---

### MODULO `net.rs`

Il modulo `net.rs` gestisce tutte le comunicazioni di rete tra client e server.  
Si basa su **Tokio** per la concorrenza asincrona e su **Framed + LengthDelimitedCodec** per inviare/ricevere messaggi serializzati JSON.  

I messaggi inviati e ricevuti sono definiti in `shared::{ClientMsg, ServerMsg}`, e i risultati vengono trasformati in `AppResult` per essere elaborati dal frontend.

---

#### Funzioni principali

##### `send_and_receive(addr: &str, msg: ClientMsg) -> Result<ServerMsg>`
- Funzione helper asincrona per inviare un messaggio al server e riceverne uno di risposta.
- Workflow:
  1. Connette un `TcpStream` all’indirizzo del server.
  2. Avvolge lo stream con `Framed` e `LengthDelimitedCodec`.
  3. Serializza il messaggio con `serde_json` e lo invia.
  4. Attende un messaggio in risposta.
  5. Deserializza la risposta in `ServerMsg`.
- Se il server non risponde, restituisce un messaggio di errore predefinito.

---

##### `listen_background(addr: String, token: Uuid, tx_app: Sender<AppResult>)`
- Listener continuo in background per ricevere **push dal server**.
- Funziona in loop infinito finché la connessione rimane aperta.
- Invia eventi al frontend tramite `mpsc::Sender<AppResult>`:
  - `PushGroupUpdated` → `AppResult::PushGroupListUpdated`
  - `PushPrivateChatListUpdated` → `AppResult::PushPrivateChatListUpdated`
  - `PushNewMessage { message, chat_id, group_id }` → `AppResult::PushNewMessage`
- Gestisce la connessione e la deserializzazione dei messaggi.
- Garantisce che i messaggi arrivino in tempo reale all’interfaccia senza polling manuale.

---

#### Autenticazione

##### `login(addr, username, password) -> AppResult`
- Invio di `ClientMsg::Login` al server.
- Converte `ServerMsg::LoginOk` in `AppResult::LoginSuccess`.
- Gestisce errori di login.

##### `register(addr, username, password) -> AppResult`
- Invio di `ClientMsg::Register`.
- Converte `ServerMsg::Registered` in `AppResult::RegisterSuccess`.
- Gestisce errori di registrazione.

---

#### Ricerca utenti

##### `search_users(addr, token, query) -> AppResult`
- Invia `ClientMsg::SearchUsers` con token di sessione e query.
- Converte `ServerMsg::UsersFound` in `AppResult::SearchResults`.
- Consente di cercare altri utenti sul server.

---

#### Chat private

##### `start_private_chat(addr, token, other_username) -> AppResult`
- Invia `ClientMsg::StartPrivateChat` per avviare una nuova chat privata.
- Converte la risposta in `AppResult::PrivateChatStarted`.

##### `get_private_chats(addr, token) -> AppResult`
- Recupera la lista delle chat private dell’utente.
- Converte `ServerMsg::PrivateChats` in `AppResult::PrivateChatsLoaded`.

##### `get_private_messages(addr, token, chat_id, limit) -> AppResult`
- Richiede gli ultimi messaggi di una chat privata.
- Converte `ServerMsg::PrivateChatMessages` in `AppResult::MessagesLoaded`.

##### `send_private_message(addr, token, chat_id, content) -> AppResult`
- Invia un messaggio a una chat privata.
- Converte `ServerMsg::PrivateMessageSent` in `AppResult::MessageSent`.

---

#### Gestione gruppi

##### `create_group(addr, token, name) -> AppResult`
- Crea un nuovo gruppo sul server.
- Converte `ServerMsg::GroupCreated` in `AppResult::GroupCreated`.

##### `add_group_member(addr, token, group_id, username) -> AppResult`
- Aggiunge un membro a un gruppo esistente.
- Converte `ServerMsg::GroupMemberAdded` in `AppResult::MemberAdded`.

##### `get_groups(addr, token) -> AppResult`
- Recupera tutti i gruppi a cui l’utente appartiene.
- Converte `ServerMsg::Groups` in `AppResult::GroupsLoaded`.

##### `get_group_members(addr, token, group_id) -> AppResult`
- Recupera la lista dei membri di un gruppo.
- Converte `ServerMsg::GroupMembers` in `AppResult::GroupMembersLoaded`.

##### `get_group_messages(addr, token, group_id, limit) -> AppResult`
- Recupera gli ultimi messaggi di un gruppo.
- Converte `ServerMsg::GroupMessages` in `AppResult::MessagesLoaded`.

##### `send_group_message(addr, token, group_id, content) -> AppResult`
- Invia un messaggio a un gruppo.
- Converte `ServerMsg::GroupMessageSent` in `AppResult::MessageSent`.

---

#### Flusso di comunicazione `net.rs` → `app.rs`

1. **Richieste sincrone/asincrone dall’app**
   - L’interfaccia chiama le funzioni di `net.rs` (`login`, `get_private_chats`, `send_private_message`, ecc.) tramite il runtime Tokio.
   - Ogni funzione invia un `ClientMsg` al server e riceve un `ServerMsg`.
   - Il messaggio di risposta viene convertito in un `AppResult` e inviato al canale interno dell’app (`tx_result`).

2. **Listener in background**
   - Una volta effettuato il login, `listen_background` rimane connesso al server.
   - Riceve messaggi push in tempo reale:
     - Aggiornamenti chat e gruppi (`PushGroupUpdated`, `PushPrivateChatListUpdated`)
     - Nuovi messaggi (`PushNewMessage`)
   - Ogni evento viene trasformato in `AppResult` e spedito al canale `tx_result` dell’app.

3. **Ricezione ed elaborazione in `update`**
   - `App::update` legge dal canale `rx_result` tutti gli `AppResult` disponibili.
   - Aggiorna lo stato della UI, liste chat/gruppi, messaggi e membri in base al contenuto di ogni evento.
   - Eventi come nuovi messaggi o aggiornamenti di gruppo vengono gestiti immediatamente, garantendo **UI reattiva senza polling continuo**.

4. **Aggiornamento della UI**
   - Dopo aver processato i risultati, `update` richiama `ctx.request_repaint_after` per ridisegnare la UI.
   - Tutti i cambiamenti dello stato dell’app (nuovi messaggi, aggiornamenti chat/gruppo, login, logout) si riflettono immediatamente nella visualizzazione.

---

##### Operazioni 

###### 1. Registrazione utente

1. L’utente inserisce **username** e **password** nella schermata di registrazione (`Register`).
2. `App::update` chiama `net::register(server_addr, username, password)` tramite Tokio.
3. `net::register` invia un messaggio `ClientMsg::Register` al server.
4. Il server riceve il messaggio e crea l’utente:
   - Se la registrazione ha successo → invia `ServerMsg::Registered`.
   - Se fallisce (username già presente) → invia `ServerMsg::Error`.
5. `net::register` riceve la risposta, la converte in `AppResult` (`RegisterSuccess` o `Error`) e la invia tramite `tx_result`.
6. `App::update` legge il risultato dal canale `rx_result` e aggiorna la UI:
   - `RegisterSuccess` → mostra messaggio di conferma.
   - `Error` → visualizza il messaggio di errore.

---

###### 2. Login utente

1. L’utente inserisce **username** e **password** nella schermata di login (`Login`).
2. `App::update` chiama `net::login(server_addr, username, password)`.
3. `net::login` invia `ClientMsg::Login` al server.
4. Il server verifica le credenziali:
   - Se corrette → invia `ServerMsg::LoginOk { session_token, user_id, username }`.
   - Se errate → invia `ServerMsg::Error`.
5. `net::login` riceve la risposta, converte in `AppResult::LoginSuccess` o `AppResult::Error` e invia tramite `tx_result`.
6. `App::update` aggiorna lo stato:
   - Imposta `session_token`, `user_id`, `username`.
   - Cambia vista in `ChatList`.
   - Avvia `listen_background` per ricevere messaggi push in tempo reale.

---

###### 3. Ricerca utente e creazione chat privata

1. L’utente inserisce una query di ricerca nella UI (`search_query`).
2. `App::update` chiama `net::search_users(server_addr, token, query)`.
3. `net::search_users` invia `ClientMsg::SearchUsers` al server.
4. Il server cerca utenti corrispondenti e risponde con `ServerMsg::UsersFound { users }`.
5. `net::search_users` converte la risposta in `AppResult::SearchResults` e invia tramite `tx_result`.
6. `App::update` aggiorna la UI mostrando i risultati nella lista `search_results`.
7. L’utente seleziona un utente e avvia una chat privata:
   - `App::update` chiama `net::start_private_chat(token, other_username)`.
   - `net::start_private_chat` invia `ClientMsg::StartPrivateChat`.
   - Il server crea la chat privata e risponde con `ServerMsg::PrivateChatStarted { chat_id }`.
   - `AppResult::PrivateChatStarted` aggiorna la lista `private_chats` e apre la chat.

---

###### 4. Scambio di messaggi in chat privata

1. L’utente scrive un messaggio (`message_input`) e preme **Invio**.
2. `App::update` chiama `net::send_private_message(token, chat_id, content)`.
3. `net::send_private_message` invia `ClientMsg::SendPrivateMessage` al server.
4. Il server memorizza il messaggio e invia `ServerMsg::PrivateMessageSent`.
5. `AppResult::MessageSent` aggiorna lo stato dell’app, pulendo l’input dell’utente.
6. Se altri utenti sono nella chat, `listen_background` riceve `ServerMsg::PushNewMessage`:
   - Converte in `AppResult::PushNewMessage`.
   - `App::update` aggiunge il messaggio in `current_messages` se la chat è aperta.

---

###### 5. Creazione di un gruppo

1. L’utente inserisce il nome del gruppo (`new_group_name`) nella UI e conferma.
2. `App::update` chiama `net::create_group(token, name)`.
3. `net::create_group` invia `ClientMsg::CreateGroup` al server.
4. Il server crea il gruppo e risponde con `ServerMsg::GroupCreated { group_id }`.
5. `AppResult::GroupCreated` aggiorna la lista `groups` e pulisce il campo `new_group_name`.
6. L’utente può aggiungere membri con `net::add_group_member(token, group_id, username)`:
   - Ogni aggiunta invia `ClientMsg::AddGroupMember`.
   - Il server risponde con `ServerMsg::GroupMemberAdded`.
   - `AppResult::MemberAdded` aggiorna lo stato e la UI del gruppo.

---

###### 6. Scambio di messaggi in chat di gruppo

1. L’utente scrive un messaggio (`message_input`) nella chat di gruppo aperta e preme **Invio**.
2. `App::update` chiama `net::send_group_message(token, group_id, content)`.
3. `net::send_group_message` invia `ClientMsg::SendGroupMessage` al server.
4. Il server memorizza il messaggio e risponde con `ServerMsg::GroupMessageSent`.
5. `AppResult::MessageSent` viene elaborato da `App::update`, che:
   - Pulisce il campo `message_input`.
   - Mantiene la chat aggiornata con i messaggi esistenti.
6. Tutti i membri del gruppo collegati ricevono `ServerMsg::PushNewMessage { message, chat_id: None, group_id }` tramite il listener in background (`listen_background`).
7. `net.rs` converte il messaggio push in `AppResult::PushNewMessage` e lo invia ad `app.rs`.
8. `App::update` aggiunge il nuovo messaggio a `current_messages` **solo se la chat di gruppo corrispondente è aperta**, aggiornando in tempo reale la UI.

---

###### 7. Logout utente

1. L’utente preme il bottone **Logout** nella UI.
2. `App::update` chiama `App::logout()`.
3. `App::logout` esegue le seguenti operazioni:
   - Reset di `session_token`, `user_id` e `username`.
   - Svuota le liste `private_chats`, `groups`, `current_messages`, `current_group_members` e `search_results`.
   - Reimposta i flag `last_search_completed` e `chats_loaded`.
   - Ricrea i canali `mpsc::Sender` e `Receiver` per i risultati asincroni.
   - Cambia vista in `Home`.
4. L’interfaccia grafica si aggiorna mostrando la schermata iniziale (`Home`) e rimuovendo tutte le informazioni private dell’utente.
5. Eventuali listener in background rimangono attivi, ma non hanno più accesso al `session_token`, quindi non possono ricevere messaggi push per l’utente disconnesso.

## SERVER

### MODULO `storage.rs`

Il file `storage.rs` gestisce **la persistenza dei dati** dell’applicazione tramite **SQLite**.  
Si occupa di memorizzare e recuperare utenti, sessioni, chat private, gruppi e messaggi.  
Il modulo utilizza le librerie principali:

- **rusqlite** → interfaccia con il database SQLite.
- **uuid** → generazione e gestione di identificatori univoci.
- **crate::errors** → definizione dei tipi `AppError` e `AppResult` per gestione degli errori.

---

#### Struct `SqliteStorage`

Rappresenta il database SQLite e incapsula la connessione:

- `conn: Connection` → connessione attiva al database SQLite.

##### Funzioni principali di `SqliteStorage`

###### `pub fn new(db_path: &str) -> AppResult<Self>`
- Costruisce un nuovo `SqliteStorage` aprendo una connessione a `db_path`.
- Parametri:
  - `db_path` → percorso del file SQLite.
- Ritorna un `AppResult` contenente `SqliteStorage` o un errore.

###### `pub fn init(db_path: &str) -> AppResult<()>`
- Inizializza il database creando le tabelle necessarie se non esistono.
- Tabelle principali: `users`, `sessions`, `private_chats`, `groups`, `group_members`, `messages`.
- Parametri:
  - `db_path` → percorso del file SQLite.
- Ritorna `AppResult<()>` indicando successo o errore.

---

#### Gestione utenti

###### `pub fn insert_user(&self, username: &str, pwd_hash: &str) -> AppResult<Uuid>`
- Inserisce un nuovo utente nel database.
- Parametri:
  - `username` → nome utente da registrare.
  - `pwd_hash` → hash della password.
- Ritorna `Uuid` dell’utente creato o errore se l’utente esiste già.

###### `pub fn get_pwd_hash(&self, username: &str) -> AppResult<Option<String>>`
- Recupera l’hash della password di un utente.
- Parametri:
  - `username` → nome utente.
- Ritorna `Some(hash)` se l’utente esiste, `None` altrimenti.

###### `pub fn get_user_id(&self, username: &str) -> AppResult<Option<Uuid>>`
- Recupera l’ID dell’utente dato il nome.
- Parametri:
  - `username` → nome utente.
- Ritorna `Some(Uuid)` se esiste, `None` altrimenti.

###### `pub fn get_username(&self, user_id: &Uuid) -> AppResult<Option<String>>`
- Recupera il nome utente dato l’ID.
- Parametri:
  - `user_id` → identificativo utente.
- Ritorna `Some(username)` se esiste, `None` altrimenti.

###### `pub fn search_users(&self, my_id: &Uuid, query: &str, limit: u32) -> AppResult<Vec<(Uuid, String)>>`
- Cerca altri utenti il cui username **inizia per** la stringa `query`, escludendo se stessi.
- Parametri:
  - `my_id` → ID dell’utente che esegue la ricerca.
  - `query` → stringa di ricerca.
  - `limit` → numero massimo di risultati.
- Ritorna un vettore di tuple `(Uuid, username)`.

---

#### Gestione sessioni

###### `pub fn insert_session(&self, username: &str, ttl_secs: i64) -> AppResult<Uuid>`
- Crea una nuova sessione per un utente.
- Parametri:
  - `username` → nome utente.
  - `ttl_secs` → durata della sessione in secondi.
- Ritorna `Uuid` del token di sessione.

###### `pub fn validate_session(&self, token: &Uuid) -> AppResult<Option<Uuid>>`
- Verifica se una sessione è valida (non scaduta).
- Parametri:
  - `token` → token di sessione.
- Ritorna `Some(user_id)` se valido, `None` se scaduto o inesistente.

---

#### Gestione chat private

###### `pub fn create_private_chat(&self, user1_id: &Uuid, user2_id: &Uuid) -> AppResult<Uuid>`
- Crea una chat privata tra due utenti.
- Parametri:
  - `user1_id`, `user2_id` → ID degli utenti.
- Ritorna l’ID della chat creata o esistente.

###### `pub fn get_user_private_chats(&self, user_id: &Uuid) -> AppResult<Vec<(Uuid, Uuid, String)>>`
- Recupera tutte le chat private di un utente.
- Parametri:
  - `user_id` → ID utente.
- Ritorna un vettore di `(chat_id, other_user_id, other_username)`.

###### `pub fn is_user_in_private_chat(&self, user_id: &Uuid, chat_id: &Uuid) -> AppResult<bool>`
- Controlla se un utente fa parte di una chat privata.
- Parametri:
  - `user_id` → ID utente.
  - `chat_id` → ID chat.
- Ritorna `true` se membro, `false` altrimenti.

###### `pub fn get_private_chat_members(&self, chat_id: &Uuid) -> AppResult<Option<(Uuid, Uuid)>>`
- Restituisce gli ID dei due membri di una chat privata.
- Parametri:
  - `chat_id` → ID chat.
- Ritorna `Some((user1, user2))` o `None`.

---

#### Gestione gruppi

###### `pub fn create_group(&self, name: &str, creator_id: &Uuid) -> AppResult<Uuid>`
- Crea un nuovo gruppo e aggiunge il creatore come membro.
- Parametri:
  - `name` → nome del gruppo.
  - `creator_id` → ID dell’utente creatore.
- Ritorna l’ID del gruppo.

###### `pub fn add_group_member(&self, group_id: &Uuid, user_id: &Uuid) -> AppResult<()>`
- Aggiunge un utente a un gruppo esistente.
- Parametri:
  - `group_id` → ID del gruppo.
  - `user_id` → ID utente.
- Ritorna `Ok(())` se successo.

###### `pub fn is_user_in_group(&self, user_id: &Uuid, group_id: &Uuid) -> AppResult<bool>`
- Controlla se un utente è membro di un gruppo.
- Parametri:
  - `user_id` → ID utente.
  - `group_id` → ID gruppo.
- Ritorna `true` o `false`.

###### `pub fn get_user_groups(&self, user_id: &Uuid) -> AppResult<Vec<(Uuid, String)>>`
- Recupera tutti i gruppi a cui un utente appartiene.
- Parametri:
  - `user_id` → ID utente.
- Ritorna un vettore di `(group_id, group_name)`.

###### `pub fn get_group_members(&self, group_id: &Uuid) -> AppResult<Vec<(Uuid, String)>>`
- Recupera tutti i membri di un gruppo.
- Parametri:
  - `group_id` → ID gruppo.
- Ritorna un vettore di `(user_id, username)`.

---

#### Gestione messaggi

###### `pub fn insert_message(&self, sender_id: &Uuid, content: &str, private_chat_id: Option<&Uuid>, group_id: Option<&Uuid>) -> AppResult<Uuid>`
- Inserisce un messaggio nel database.
- Parametri:
  - `sender_id` → ID dell’utente mittente.
  - `content` → testo del messaggio.
  - `private_chat_id` → ID della chat privata (opzionale).
  - `group_id` → ID del gruppo (opzionale).
- Ritorna l’ID del messaggio inserito.

###### `pub fn get_private_chat_messages(&self, chat_id: &Uuid, limit: u32) -> AppResult<Vec<(Uuid, Uuid, String, String, i64)>>`
- Recupera i messaggi di una chat privata.
- Parametri:
  - `chat_id` → ID chat.
  - `limit` → numero massimo di messaggi.
- Ritorna un vettore di `(msg_id, sender_id, sender_username, content, sent_at)`.

###### `pub fn get_group_messages(&self, group_id: &Uuid, limit: u32) -> AppResult<Vec<(Uuid, Uuid, String, String, i64)>>`
- Recupera i messaggi di un gruppo.
- Parametri:
  - `group_id` → ID gruppo.
  - `limit` → numero massimo di messaggi.
- Ritorna un vettore di `(msg_id, sender_id, sender_username, content, sent_at)`.

---

#### Funzioni di supporto

###### `fn is_unique_violation(e: &rusqlite::Error) -> bool`
- Controlla se un errore SQLite è dovuto a violazione di vincolo `UNIQUE`.

###### `fn now_unix() -> i64`
- Restituisce il timestamp corrente in secondi dall’epoca Unix.

---

### MODULO `handlers.rs`

Il file `handlers.rs` gestisce **la logica applicativa lato server** dell’applicazione.  
Si occupa di processare le richieste dei client, interagire con il database tramite `SqliteStorage` e inviare notifiche tramite `PeerMap`.  

Utilizza:

- `SqliteStorage` → per persistere e recuperare dati da SQLite.
- `auth` → per hashing e verifica delle password.
- `PeerMap` → struttura condivisa con le connessioni attive dei client.
- `shared` → tipi condivisi come `UserInfo`, `PrivateChatInfo`, `GroupInfo`, `MessageInfo`, `ServerMsg`.

---

#### Autenticazione e ricerca utenti

##### `pub fn handle_register(db: &SqliteStorage, username: String, password: String) -> AppResult<Uuid>`
- Registra un nuovo utente.
- Parametri:
  - `db` → riferimento a `SqliteStorage`.
  - `username` → nome utente da registrare.
  - `password` → password in chiaro.
- Obiettivo: crea l’utente con password hashata e restituisce il suo `Uuid`.
- Validazioni: input non vuoti.

##### `pub fn handle_login(db: &SqliteStorage, username: String, password: String) -> AppResult<(Uuid, Uuid, String)>`
- Effettua il login di un utente.
- Parametri:
  - `db` → database.
  - `username`, `password` → credenziali.
- Ritorna una tupla `(token_sessione, user_id, username)`.
- Validazioni: verifica password hashata, esistenza utente.

##### `pub fn handle_search_users(db: &SqliteStorage, token: Uuid, query: String) -> AppResult<Vec<UserInfo>>`
- Cerca utenti registrati che iniziano con `query`, escludendo l’utente corrente.
- Parametri:
  - `token` → token di sessione dell’utente.
  - `query` → stringa di ricerca.
- Ritorna un vettore di `UserInfo`.

---

#### Chat private

##### `pub fn handle_start_private_chat(db: &SqliteStorage, peers: &PeerMap, token: Uuid, other_username: String) -> AppResult<Uuid>`
- Avvia una chat privata tra l’utente corrente e un altro utente.
- Parametri:
  - `peers` → mappa dei client connessi per notifiche.
  - `other_username` → destinatario.
- Ritorna `chat_id`.
- Notifica in tempo reale il destinatario tramite `PeerMap`.

##### `pub fn handle_get_private_chats(db: &SqliteStorage, token: Uuid) -> AppResult<Vec<PrivateChatInfo>>`
- Recupera le chat private dell’utente.
- Parametri:
  - `token` → token sessione.
- Ritorna un vettore di `PrivateChatInfo`.

##### `pub fn handle_get_private_chat_messages(db: &SqliteStorage, token: Uuid, chat_id: Uuid, limit: u32) -> AppResult<Vec<MessageInfo>>`
- Recupera messaggi di una chat privata.
- Parametri:
  - `chat_id` → ID chat privata.
  - `limit` → numero massimo di messaggi.
- Validazioni: l’utente deve appartenere alla chat.
- Ritorna un vettore di `MessageInfo`.

##### `pub fn handle_send_private_message(db: &SqliteStorage, peers: &PeerMap, token: Uuid, chat_id: Uuid, content: String) -> AppResult<Uuid>`
- Invia un messaggio in una chat privata.
- Parametri:
  - `content` → testo del messaggio.
- Validazioni: non vuoto, utente membro della chat.
- Notifica in tempo reale l’altro partecipante tramite `PeerMap`.
- Ritorna `message_id`.

---

#### Gestione gruppi

##### `pub fn handle_create_group(db: &SqliteStorage, token: Uuid, name: String) -> AppResult<Uuid>`
- Crea un nuovo gruppo.
- Parametri:
  - `name` → nome del gruppo.
- Validazioni: nome non vuoto.
- Ritorna `group_id`.

##### `pub fn handle_add_group_member(db: &SqliteStorage, peers: &PeerMap, token: Uuid, group_id: Uuid, username: String) -> AppResult<()>`
- Aggiunge un membro a un gruppo esistente.
- Parametri:
  - `username` → utente da aggiungere.
- Validazioni:
  - Solo membri possono aggiungere altri.
  - Non auto-aggiunta.
  - Non duplicati.
- Notifica in tempo reale il nuovo membro tramite `PeerMap`.

##### `pub fn handle_get_groups(db: &SqliteStorage, token: Uuid) -> AppResult<Vec<GroupInfo>>`
- Recupera i gruppi a cui l’utente appartiene.
- Ritorna un vettore di `GroupInfo`.

##### `pub fn handle_get_group_members(db: &SqliteStorage, token: Uuid, group_id: Uuid) -> AppResult<Vec<UserInfo>>`
- Recupera i membri di un gruppo.
- Parametri:
  - `group_id` → ID gruppo.
- Validazioni: utente deve essere membro.
- Ritorna un vettore di `UserInfo`.

##### `pub fn handle_get_group_messages(db: &SqliteStorage, token: Uuid, group_id: Uuid, limit: u32) -> AppResult<Vec<MessageInfo>>`
- Recupera messaggi di un gruppo.
- Parametri:
  - `group_id` → ID gruppo.
  - `limit` → numero massimo di messaggi.
- Validazioni: utente deve essere membro.
- Ritorna un vettore di `MessageInfo`.

##### `pub fn handle_send_group_message(db: &SqliteStorage, peers: &PeerMap, token: Uuid, group_id: Uuid, content: String) -> AppResult<Uuid>`
- Invia un messaggio in un gruppo.
- Parametri:
  - `content` → testo del messaggio.
- Validazioni: non vuoto, utente membro.
- Notifica tutti i membri connessi tranne il mittente tramite `PeerMap`.
- Ritorna `message_id`.

---

#### Note generali

- Tutte le funzioni che richiedono autenticazione verificano il token tramite `db.validate_session`.
- Tutte le funzioni restituiscono `AppResult` per una gestione centralizzata degli errori.
- `PeerMap` permette di inviare notifiche in tempo reale ai client connessi tramite `ServerMsg`.

---

### MODULO `auth.rs`

Il file `auth.rs` gestisce **l’autenticazione e la sicurezza delle password** lato server.  
Fornisce funzionalità per creare hash sicuri delle password e verificarle durante il login.

---

#### Librerie utilizzate

- **argon2** → algoritmo di hashing sicuro per password.
  - `PasswordHash`, `PasswordHasher`, `PasswordVerifier`, `SaltString` → strutture per generare e verificare hash.
  - `Argon2` → implementazione concreta dell’algoritmo Argon2.
- **rand** → generazione di salt casuali per l’hash.
- `AppResult` / `AppError` → tipi personalizzati per gestione errori.

---

#### Funzioni principali

##### `pub fn hash_password(password: &str) -> AppResult<String>`
- Obiettivo: generare un hash sicuro della password.
- Parametri:
  - `password` → password in chiaro da proteggere.
- Funzionamento:
  1. Genera un **salt casuale** tramite `SaltString`.
  2. Applica **Argon2** per calcolare l’hash.
  3. Converte l’hash in stringa e lo restituisce.
- Restituisce `AppResult<String>` contenente l’hash o un errore.

##### `pub fn verify_password(password: &str, stored_hash: &str) -> AppResult<bool>`
- Obiettivo: verificare se una password fornita corrisponde all’hash memorizzato.
- Parametri:
  - `password` → password in chiaro da verificare.
  - `stored_hash` → hash memorizzato nel database.
- Funzionamento:
  1. Parsea l’hash memorizzato con `PasswordHash::new`.
  2. Verifica la password con `Argon2::verify_password`.
  3. Restituisce `true` se la password corrisponde, `false` altrimenti.
- Restituisce `AppResult<bool>`.

---

#### Note generali

- Tutte le password salvate nel database devono essere hashate tramite `hash_password`.
- Durante il login, `verify_password` garantisce che la password fornita corrisponda all’hash memorizzato senza mai esporre la password in chiaro.
- L’uso di Argon2 con salt casuale assicura **resistenza agli attacchi di dizionario e rainbow table**.

---

### MODULO `errors.rs`

Il file `errors.rs` gestisce **tutti gli errori personalizzati e tipi di risultato** dell’applicazione lato server.  
Fornisce un sistema centralizzato per rappresentare errori di validazione, autenticazione, database, crittografia, I/O e serializzazione.

---

#### Librerie utilizzate

- **thiserror** → facilita la definizione di enum per errori con messaggi leggibili.
- **rusqlite** → errori del database SQLite.
- **argon2::password_hash** → errori relativi all’hash delle password.
- **serde_json** → errori di serializzazione/deserializzazione JSON.
- **std::io** → errori di I/O generici.

---

#### Enum `AppError`

Rappresenta tutti i possibili errori dell’applicazione:

- `Validation(String)` → dati o input non validi, con messaggio descrittivo.
- `UserExists` → tentativo di registrazione di un utente già presente nel database.
- `BadCredentials` → username o password errati durante il login.
- `Db(#[from] rusqlite::Error)` → errori derivanti dalle operazioni SQLite.
- `Crypto(#[from] argon2::password_hash::Error)` → errori di hashing/verifica password.
- `Serde(#[from] serde_json::Error)` → errori di serializzazione o deserializzazione JSON.
- `Io(#[from] std::io::Error)` → errori di input/output.

> L’attributo `#[from]` permette di convertire automaticamente errori provenienti da librerie esterne in `AppError`.

---

#### Tipo `AppResult<T>`

- Alias per `Result<T, AppError>`.
- Usato come tipo di ritorno standard per tutte le funzioni lato server.
- Garantisce una gestione coerente degli errori in tutto il codice.

---

### MODULO `net.rs`

Il file `net.rs` gestisce **le connessioni TCP lato server**, la ricezione dei messaggi dai client, l’inoltro ai gestori (`handlers`) e la gestione dei messaggi push verso i client connessi.

---

#### Librerie utilizzate

- **futures_util::SinkExt, StreamExt** → utilities per lavorare con stream asincroni.
- **tokio::net::TcpStream** → gestione delle connessioni TCP asincrone.
- **tokio_util::codec::{Framed, LengthDelimitedCodec}** → per incapsulare i dati TCP come frame delimitati da lunghezza fissa.
- **tokio::sync::mpsc** → canali asincroni per messaggi push verso i client.
- **uuid::Uuid** → identificatori univoci per utenti, chat e gruppi.
- **serde_json** → serializzazione e deserializzazione JSON dei messaggi.
- Moduli locali: `handlers`, `storage::SqliteStorage`, `errors::AppError`, `PeerMap`.

---

#### Funzioni principali

##### `serve_connection(sock: TcpStream, db_path: &'static str, peers: PeerMap)`

- **Descrizione:** gestisce una singola connessione TCP con un client.
- **Parametri:**
  - `sock` → la connessione TCP con il client.
  - `db_path` → percorso del database SQLite.
  - `peers` → mappa condivisa dei client connessi per invio push.
- **Funzionamento:**
  1. Avvia un framed socket con `LengthDelimitedCodec`.
  2. Crea un’istanza di `SqliteStorage`.
  3. Crea un canale mpsc per i messaggi push.
  4. Loop principale:
     - Riceve messaggi dai client (`ClientMsg`) e li inoltra a `handle_client_msg`.
     - Riceve messaggi push dal server e li invia al client.
  5. Gestisce la disconnessione sicura, rimuovendo il client dalla mappa `peers` solo se non è connesso altrove.

---

##### `handle_client_msg(msg: &ClientMsg, db: &SqliteStorage, peers: &PeerMap) -> ServerMsg`

- **Descrizione:** mappa i messaggi ricevuti dai client ai gestori appropriati (`handlers`) e restituisce la risposta `ServerMsg`.
- **Parametri:**
  - `msg` → messaggio ricevuto dal client.
  - `db` → riferimento allo storage SQLite.
  - `peers` → mappa condivisa dei client connessi.
- **Funzionamento:** 
  - Usa un match sul tipo di `ClientMsg`.
  - Per ogni messaggio chiama la funzione del modulo `handlers` corrispondente.
  - Converte eventuali errori in messaggi di errore leggibili tramite `map_error`.

---

##### `map_error(e: AppError) -> ServerMsg`

- **Descrizione:** converte gli errori `AppError` in messaggi leggibili per il client (`ServerMsg::Error`).
- **Parametri:**
  - `e` → errore da mappare.
- **Funzionamento:** assegna un messaggio leggibile a seconda del tipo di errore (es. `UserExists` → "Username già in uso").

---

##### `send(framed: &mut Framed<TcpStream, LengthDelimitedCodec>, msg: &ServerMsg) -> Result<(), std::io::Error>`

- **Descrizione:** invia un messaggio `ServerMsg` serializzato in JSON attraverso la connessione TCP.
- **Parametri:**
  - `framed` → riferimento al framed socket TCP.
  - `msg` → messaggio da inviare.
- **Funzionamento:** serializza `msg` in JSON e lo invia come frame delimitato.

---

#### Note sul funzionamento

- **Connessioni asincrone:** tutte le operazioni di lettura/scrittura TCP sono asincrone usando Tokio.
- **Push dei messaggi:** tramite canali mpsc unicast verso i client registrati nella mappa `peers`.
- **Sicurezza della sessione:** ogni client deve registrarsi per il push (`ClientMsg::Listen`) con un token valido.
- **Gestione multi-connessione:** se un utente si riconnette, la vecchia sessione non viene rimossa se diversa dalla connessione corrente.

---

#### Messaggi supportati

Il modulo gestisce tutti i messaggi definiti in `ClientMsg`:

- Registrazione/login (`Register`, `Login`)
- Ricerca utenti (`SearchUsers`)
- Chat private (`StartPrivateChat`, `GetPrivateChats`, `GetPrivateChatMessages`, `SendPrivateMessage`)
- Gruppi (`CreateGroup`, `AddGroupMember`, `GetGroups`, `GetGroupMembers`, `GetGroupMessages`, `SendGroupMessage`)
- Messaggi non gestiti vengono restituiti come errore generico.

---

### MODULO `main.rs`

Questo file rappresenta il punto di ingresso del server scritto in Rust.  
Il suo compito è:

- inizializzare il database,
- avviare un thread dedicato al monitoring,
- aprire una porta TCP,
- accettare le connessioni in arrivo,
- passare ogni connessione a un handler asincrono,
- mantenere una mappa condivisa degli utenti connessi per gestire i messaggi push.

---

#### 1. Moduli interni importati

Vengono importati i moduli locali che contengono:
- **errors**: gestione degli errori personalizzati del server.
- **storage**: gestione del database SQLite.
- **auth**: funzioni di autenticazione.
- **handlers**: gestori delle varie operazioni richieste dai client.
- **net**: funzioni di rete, incluso `serve_connection`.
- **monitoring**: modulo dedicato al monitoraggio del server.

---

#### 2. Costanti e tipi principali

- `DB_PATH`: percorso del database SQLite.
- `PeerMap`: una mappa condivisa e thread-safe che associa ogni `Uuid` (utente) a un canale `mpsc::UnboundedSender`.  
  Serve per inviare messaggi push in tempo reale ai client connessi.

L’uso combinato di `Arc` (reference counting thread-safe) e `Mutex` (mutua esclusione) permette a più task Tokio di condividere e modificare questa mappa.

---

#### 3. Funzione principale asincrona

La funzione `main` usa l’attributo `#[tokio::main]`, quindi tutto gira su un runtime Tokio asincrono.

##### 3.1 Avvio del monitoring
Viene lanciato un thread separato tramite `spawn_blocking` per eseguire il monitoring del server.  
Questo impedisce che il monitoring blocchi il runtime asincrono di Tokio.

---

##### 3.2 Inizializzazione del database
`SqliteStorage::init(DB_PATH)` crea le tabelle necessarie (se non esistono).  
Questa fase funge da “bootstrap” del database.

---

##### 3.3 Creazione del listener TCP
Il server si mette in ascolto sulla porta `7878` su tutte le interfacce (`0.0.0.0`).  
Il listener è asincrono grazie a `tokio::net::TcpListener`.

---

##### 3.4 Creazione della PeerMap condivisa
`peers` è un `Arc<Mutex<HashMap<...>>>`, quindi può essere clonato e condiviso tra tutte le connessioni.  
Ogni client connesso potrà essere registrato in questa mappa, permettendo invii di messaggi verso client specifici.

---

##### 3.5 Loop principale delle connessioni
Il server entra in un ciclo infinito che:

1. attende una nuova connessione (`listener.accept().await`),
2. clona la mappa condivisa,
3. avvia un task Tokio dedicato per gestire la singola connessione:  
   `tokio::spawn(serve_connection(socket, DB_PATH, peers))`.

Ogni connessione viene gestita in parallelo e indipendentemente dalle altre.

`serve_connection` si occupa di:
- handshake,
- autenticazione,
- lettura/scrittura dei messaggi,
- eventuale registrazione nella PeerMap.

---

#### Riassunto

In sintesi, il `main`:

- Avvia un thread per il monitoring,
- Inizializza il DB SQLite,
- Ascolta nuove connessioni TCP,
- Gestisce ogni client in un task Tokio dedicato,
- Mantiene una mappa globale e thread-safe per invio di messaggi ai client.

---

#### Flusso generico di comunicazione tra client e server

1. **Connessione**  
   - Il client si connette al server TCP (`TcpStream::connect`) e instaura un canale framed (`Framed::new`) per inviare/ricevere messaggi.
   - Sul server (`main.rs`), `TcpListener::bind` accetta nuove connessioni e per ciascuna avvia `serve_connection` in un task separato (`tokio::spawn`).

2. **Ascolto e invio messaggi**  
   - Il server utilizza un ciclo `loop` con `tokio::select!` per ascoltare sia i messaggi dal client (`framed.next()`) sia eventuali messaggi push destinati al client (`rx_push.recv()`).
   - La mappa `PeerMap` (`Arc<Mutex<HashMap<Uuid, mpsc::UnboundedSender<ServerMsg>>>>`) mantiene per ogni utente online un canale `mpsc` su cui inviare notifiche push (nuovi messaggi, aggiornamenti gruppi, ecc.).
   - Il client, dopo il login, avvia un listener in background (`listen_background`) che rimane in ascolto su `ServerMsg` push e invia gli eventi ricevuti all’interfaccia tramite `mpsc::Sender<AppResult>`.

3. **Gestione messaggi**  
   - I messaggi ricevuti dal client vengono deserializzati in `ClientMsg`.
   - `net.rs` chiama `handle_client_msg`, che a sua volta invoca le funzioni di `handlers.rs` per l’autenticazione, la gestione chat e gruppi, ecc.
   - Le risposte sono inviate al client come `ServerMsg`, e in caso di utenti online, alcune azioni triggerano **push** tramite `PeerMap` (ad esempio nuove chat private o messaggi).

---

#### Flussi specifici

##### Registrazione

1. Il client invia `ClientMsg::Register { username, password }` tramite `net.rs::register()`.
2. Il server riceve il messaggio in `serve_connection`.
3. `handle_client_msg` chiama `handlers::handle_register`:
   - Controlla input vuoti
   - Effettua l’hash della password (`auth::hash_password`)
   - Inserisce l’utente nel database (`SqliteStorage::insert_user`)
4. Il server risponde con `ServerMsg::Registered` o `ServerMsg::Error`.
5. Il client aggiorna lo stato (`AppResult::RegisterSuccess`), mostra messaggio e reindirizza alla vista di login.

---

##### Login

1. Il client invia `ClientMsg::Login { username, password }` tramite `net.rs::login()`.
2. Il server gestisce il messaggio in `handle_client_msg` → `handlers::handle_login`:
   - Recupera hash password dal DB
   - Verifica con `auth::verify_password`
   - Crea sessione con token (`insert_session`) e ottiene `user_id`
3. Risposta `ServerMsg::LoginOk` viene inviata al client.
4. Il client memorizza `session_token` e avvia il listener in background (`listen_background`) per ricevere push.
5. La connessione viene aggiunta a `PeerMap` quando il client invia `ClientMsg::Listen { token }`. Questo permette al server di inviare messaggi push al client online.

---

##### Logout

1. Il client cancella sessione locale (`session_token`, `user_id`, `username`) e pulisce i dati locali (`private_chats`, `groups`, `current_messages`).
2. Il server, quando la connessione TCP si chiude, rimuove l’utente dalla `PeerMap` SOLO se il canale corrisponde alla connessione uscente. Questo evita di eliminare sessioni attive se l’utente è connesso altrove.

---

##### Ricerca utente

1. Il client invia `ClientMsg::SearchUsers { token, query }`.
2. Il server valida la sessione (`db.validate_session`) e chiama `handlers::handle_search_users`.
3. Il DB restituisce gli utenti che corrispondono alla query (escludendo l’utente corrente).
4. La risposta `ServerMsg::UsersFound` viene inviata al client, che aggiorna `search_results` e la GUI.

---

##### Creazione e invio messaggi in chat privata

1. Il client invia `ClientMsg::StartPrivateChat { token, other_username }` per creare la chat, e successivamente `ClientMsg::SendPrivateMessage { token, chat_id, content }`.
2. Il server:
   - Verifica sessione e membri chat (`validate_session`, `is_user_in_private_chat`)
   - Inserisce il messaggio nel DB (`insert_message`)
   - Recupera l’ID dell’altro utente
   - Se l’altro utente è online, invia un push su `PeerMap` con `ServerMsg::PushNewMessage`.
3. Il client riceve il push tramite `listen_background` e aggiorna la chat in tempo reale.

---

##### Creazione e invio messaggi in chat di gruppo

1. Il client invia `ClientMsg::CreateGroup { token, name }` o `ClientMsg::SendGroupMessage { token, group_id, content }`.
2. Il server:
   - Verifica sessione e membri del gruppo (`validate_session`, `is_user_in_group`)
   - Inserisce il messaggio nel DB (`insert_message`)
   - Recupera i membri del gruppo
   - Per ogni membro online (tracciato in `PeerMap`), invia push `ServerMsg::PushNewMessage` con `group_id`.
3. Il client riceve i push, e se la chat di gruppo è aperta, aggiorna `current_messages`.

---

##### Nota sull’utilizzo della `PeerMap`

- La `PeerMap` serve come **registro degli utenti online** con il loro canale di comunicazione push.
- Ogni volta che un utente logga o ascolta (`ClientMsg::Listen`), il server aggiunge un entry `user_id -> tx_push`.
- Quando un messaggio privato o di gruppo viene creato, il server verifica se i destinatari sono online in `PeerMap` e invia push in tempo reale.
- Questo meccanismo consente notifiche immediate senza che il client debba interrogare continuamente il server.

---

In sintesi, il flusso Client ↔ Server funziona così:

1. Connessione TCP → framed codec → serializzazione JSON  
2. Validazione sessione e gestione richieste tramite `handlers.rs`  
3. Risposte immediate (`ServerMsg`) + notifiche push per utenti online (`PeerMap`)  
4. Client GUI aggiorna stati locali e mostra messaggi, chat e gruppi in tempo reale.

