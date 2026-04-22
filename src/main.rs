use std::fmt;
use std::io::{BufRead, BufReader, Write, Read}; // ← Read ajouté ici pour TcpStream
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::fs::OpenOptions;
use chrono::Local;
use sysinfo::System;

// CONFIGURATION

const AUTH_TOKEN: &str = "ENSPD2026";
const SERVER_PORT: &str = "0.0.0.0:7878";


// ÉTAPE 1 : Modélisation des données (Structs + Trait Display)


#[derive(Debug, Clone)]
struct CpuInfo {
    usage_percent: f32,
    core_count: usize,
}

#[derive(Debug, Clone)]
struct MemInfo {
    total_mb: u64,
    used_mb: u64,
    free_mb: u64,
}

#[derive(Debug, Clone)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory_mb: u64,
}

#[derive(Debug, Clone)]
struct SystemSnapshot {
    timestamp: String,
    cpu: CpuInfo,
    memory: MemInfo,
    top_processes: Vec<ProcessInfo>,
}

impl fmt::Display for CpuInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CPU: {:.1}% ({} coeurs)", self.usage_percent, self.core_count)
    }
}

impl fmt::Display for MemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MEM: {}MB utilises / {}MB total ({}MB libres)",
            self.used_mb, self.total_mb, self.free_mb
        )
    }
}

impl fmt::Display for ProcessInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  [{:>6}] {:<20} CPU:{:>5.1}%  MEM:{:>4}MB",
            self.pid, self.name, self.cpu_usage, self.memory_mb
        )
    }
}

impl fmt::Display for SystemSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== SysWatch — {} ===", self.timestamp)?;
        writeln!(f, "{}", self.cpu)?;
        writeln!(f, "{}", self.memory)?;
        writeln!(f, "--- Top Processus ---")?;
        for p in &self.top_processes {
            writeln!(f, "{}", p)?;
        }
        write!(f, "")
    }
}


// ÉTAPE 2 : Gestion d'erreurs & Collecte système

#[derive(Debug)]
enum SysWatchError {
    CollectionFailed(String),
}

impl fmt::Display for SysWatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SysWatchError::CollectionFailed(msg) => write!(f, "Erreur collecte: {}", msg),
        }
    }
}
impl std::error::Error for SysWatchError {}

fn collect_snapshot() -> Result<SystemSnapshot, SysWatchError> {
    let mut sys = System::new_all();
    sys.refresh_all();
    thread::sleep(Duration::from_millis(500));
    sys.refresh_cpu_usage();

    let usage_percent = sys.global_cpu_info().cpu_usage();
    let core_count = sys.cpus().len();
    if core_count == 0 {
        return Err(SysWatchError::CollectionFailed("Aucun CPU detecte".into()));
    }

    let total_mb = sys.total_memory() / 1024 / 1024;
    let used_mb = sys.used_memory() / 1024 / 1024;
    let free_mb = sys.free_memory() / 1024 / 1024;

    let mut procs: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p| ProcessInfo {
            pid: p.pid().as_u32(),
            name: p.name().to_string(),
            cpu_usage: p.cpu_usage(),
            memory_mb: p.memory() / 1024 / 1024,
        })
        .collect();

    procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
    procs.truncate(5);

    Ok(SystemSnapshot {
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        cpu: CpuInfo { usage_percent, core_count },
        memory: MemInfo { total_mb, used_mb, free_mb },
        top_processes: procs,
    })
}

// ÉTAPE 3 : Formatage des réponses réseau


fn format_response(snapshot: &SystemSnapshot, command: &str) -> String {
    match command.trim().to_lowercase().as_str() {
        "cpu" => {
            let bar_len = (snapshot.cpu.usage_percent / 5.0) as usize;
            let bar = "#".repeat(bar_len.min(20)) + &".".repeat(20 - bar_len.min(20));
            format!("CPU:\n[{}] {:.1}%\n{}\n", bar, snapshot.cpu.usage_percent, snapshot.cpu)
        },
        "mem" => {
            let percent = (snapshot.memory.used_mb as f64 / snapshot.memory.total_mb as f64) * 100.0;
            let bar_len = (percent / 5.0) as usize;
            let bar = "#".repeat(bar_len.min(20)) + &".".repeat(20 - bar_len.min(20));
            format!("MEM:\n[{}] {:.1}%\n{}\n", bar, percent, snapshot.memory)
        },
        "ps" | "procs" => {
            let list: String = snapshot.top_processes
                .iter()
                .enumerate()
                .map(|(i, p)| format!("{}. {}\n", i + 1, p))
                .collect();
            format!("PROCESSUS:\n{}", list)
        },
        "all" | "" => format!("{}\n", snapshot),
        "help" => "Commandes: cpu, mem, ps, all, help, quit, msg <IP> <texte>\n".to_string(),
        "quit" => "Deconnexion.\n".to_string(),
        _ => format!("Commande inconnue: '{}'. Tapez 'help'.\n", command.trim()),
    }
}


// ÉTAPE 5 (Bonus) : Journalisation fichier

fn log_event(message: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{}] {}\n", timestamp, message);
    print!("{}", line);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("syswatch.log")
    {
        let _ = file.write_all(line.as_bytes());
    }
}

// MESSAGERIE P2P SUR WIFI (Bonus demandé)

fn send_message_to_peer(target_ip: &str, sender_addr: &str, message: &str) -> String {
    let target_addr = format!("{}:7878", target_ip);
    log_event(&format!("[->] Envoi vers {}", target_addr));

    // Connexion au destinataire avec timeout implicite (3s par défaut)
    match TcpStream::connect(&target_addr) {
        Ok(mut stream) => {
            // Étape 1 : s'authentifier automatiquement
            let _ = stream.write_all(format!("{}\n", AUTH_TOKEN).as_bytes());
            stream.flush().ok();
            
            // Petite pause pour laisser le temps au destinataire de répondre "OK"
            thread::sleep(Duration::from_millis(100));
            
            // Lire et ignorer la réponse d'authentification
            let mut buf = [0u8; 256];
            let _ = stream.read(&mut buf);

            // Étape 2 : envoyer le message formaté
            let payload = format!("[MSG DE {}] {}\n", sender_addr, message);
            let _ = stream.write_all(payload.as_bytes());
            stream.flush().ok();

            log_event(&format!("[✓] Message envoye a {}", target_ip));
            "Message envoye avec succes.\n".to_string()
        }
        Err(e) => {
            log_event(&format!("[x] Echec envoi a {}: {}", target_ip, e));
            format!("Erreur: Impossible de joindre {}.\n", target_ip)
        }
    }
}

// ÉTAPE 4 : Serveur TCP multi-threadé

fn handle_client(mut stream: TcpStream, snapshot: Arc<Mutex<SystemSnapshot>>) {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "inconnu".into());
    log_event(&format!("[+] Connexion de {}", peer));

    // Auth
    let _ = stream.write_all("TOKEN: ".as_bytes());
    let mut reader = BufReader::new(stream.try_clone().expect("Clone stream echoue"));
    let mut token_line = String::new();
    
    if reader.read_line(&mut token_line).is_err() || token_line.trim() != AUTH_TOKEN {
        let _ = stream.write_all("ACCES REFUSE\n".as_bytes());
        log_event(&format!("[!] Acces refuse: {}", peer));
        return;
    }
    let _ = stream.write_all("OK\nBienvenue sur SysWatch. Tapez 'help'.\n> ".as_bytes());
    log_event(&format!("[✓] Authentifie: {}", peer));

    // Boucle de commandes
    for line in reader.lines() {
        match line {
            Ok(cmd) => {
                let cmd = cmd.trim().to_string();
                log_event(&format!("[{}] commande: '{}'", peer, cmd));

                if cmd.eq_ignore_ascii_case("quit") {
                    let _ = stream.write_all("Au revoir!\n".as_bytes());
                    break;
                }

                // Gestion de la commande `msg <IP> <texte>`
                let response = if cmd.starts_with("msg ") {
                    let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
                    if parts.len() < 3 {
                        "Format: msg <IP> <message>\n".to_string()
                    } else {
                        send_message_to_peer(parts[1], &peer, parts[2])
                    }
                } else {
                    let snap = snapshot.lock().unwrap();
                    format_response(&snap, &cmd)
                };

                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(b"> ");
            }
            Err(_) => break,
        }
    }
    log_event(&format!("[-] Deconnexion: {}", peer));
}

fn snapshot_refresher(snapshot: Arc<Mutex<SystemSnapshot>>) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(5));
            if let Ok(new_snap) = collect_snapshot() {
                let mut data = snapshot.lock().unwrap();
                *data = new_snap;
                println!("[refresh] Metriques mises a jour");
            }
        }
    });
}

fn main() {
    println!("SysWatch v2.1 demarrage...");

    let initial = collect_snapshot().unwrap_or_else(|_| SystemSnapshot {
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        cpu: CpuInfo { usage_percent: 0.0, core_count: 0 },
        memory: MemInfo { total_mb: 0, used_mb: 0, free_mb: 0 },
        top_processes: vec![],
    });
    println!("Metriques initiales OK");

    let shared_snapshot = Arc::new(Mutex::new(initial));
    snapshot_refresher(Arc::clone(&shared_snapshot));

    // 0.0.0.0 = ecoute sur TOUTES les interfaces (WiFi + Local)
    let listener = TcpListener::bind(SERVER_PORT).expect("Port 7878 indisponible");
    println!("Serveur en ecoute sur {}", SERVER_PORT);
    println!("IP WiFi de cette machine : 192.168.0.105");
    println!("Connectez-vous avec: telnet 192.168.0.105 7878");
    println!("Pour envoyer un message: msg <IP_CIBLE> <votre message>");
    println!("Ctrl+C pour arreter.\n");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let snap_clone = Arc::clone(&shared_snapshot);
                thread::spawn(move || handle_client(stream, snap_clone));
            }
            Err(e) => eprintln!("Erreur connexion: {}", e),
        }
    }
}