// === FIX: Import puliti per sysinfo 0.30 (Niente più *Ext) ===
use sysinfo::{System, RefreshKind, ProcessRefreshKind, Pid}; 
// =============================================================
use std::time::Duration;
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;
use std::process;

const LOG_FILE_NAME: &str = "server_cpu_log.txt";
const LOG_INTERVAL_SECS: u64 = 120; // 2 minuti

/// Funzione di utilità per scrivere un messaggio nel file di log con timestamp.
fn write_log(message: &str) -> std::io::Result<()> {
    // Apre il file in modalità append e lo crea se non esiste
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(LOG_FILE_NAME)?;

    // Ottiene l'ora attuale e la formatta
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    writeln!(file, "[{}] {}", timestamp, message)?;

    Ok(())
}

/// Avvia il loop di monitoraggio della CPU del server.
pub fn start_monitoring() {
    // In sysinfo 0.30, from_u32 è un metodo diretto di Pid, non serve PidExt
    let pid = Pid::from_u32(process::id());

    // Configurazione Refresh: Chiediamo tutto sui processi per essere sicuri di avere la CPU
    let refresh_config = RefreshKind::new().with_processes(ProcessRefreshKind::everything());
    
    // Inizializza il System
    let mut sys = System::new_with_specifics(refresh_config);

    println!("[MONITORING] Avvio monitoraggio CPU per PID: {}", pid);

    // Attendi un istante per permettere al sistema di raccogliere il primo campione (necessario per il calcolo CPU)
    std::thread::sleep(Duration::from_millis(500));
    sys.refresh_processes();

    loop {
        // Aggiorna i dati dei processi dal sistema operativo
        // refresh_processes è ora un metodo diretto di System, non serve SystemExt
        sys.refresh_processes();

        if let Some(process) = sys.process(pid) {
            // cpu_usage è ora un metodo diretto di Process, non serve ProcessExt
            let cpu_usage = process.cpu_usage();

            let log_message = format!("CPU Usage: {:.2}%", cpu_usage);

            match write_log(&log_message) {
                Ok(_) => {
                    println!("[MONITORING] Log scritto: {}", log_message);
                },
                Err(e) => eprintln!("[MONITORING] Errore nella scrittura del log: {}", e),
            }
        } else {
            eprintln!("[MONITORING] Processo server non trovato. Interruzione del monitoraggio.");
            break;
        }

        // Aspetta 2 minuti (120 secondi)
        std::thread::sleep(Duration::from_secs(LOG_INTERVAL_SECS));
    }
}