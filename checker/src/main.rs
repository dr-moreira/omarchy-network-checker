use std::collections::HashMap;
use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use clap::Parser;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::Local;

/// Define a estrutura para os argumentos da linha de comando
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Caminho para o arquivo TOML de configuração (opcional, usa locais padrão se não fornecido)
    #[arg(short, long)]
    file: Option<String>,

    /// Executa em modo daemon, verificando continuamente
    #[arg(short, long)]
    daemon: bool,

    /// Output a single check as JSON (for status bars / scripts)
    #[arg(short, long)]
    json: bool,
}

/// Define a estrutura que corresponde ao conteúdo do arquivo TOML
#[derive(Deserialize, Debug, Clone)]
struct Config {
    servers: Vec<Server>,
    #[serde(default)]
    settings: Settings,
}

#[derive(Deserialize, Debug, Clone)]
struct Server {
    name: String,
    host: String,
    #[serde(default)]
    ports: Vec<u16>,
}

#[derive(Deserialize, Debug, Clone)]
struct Settings {
    #[serde(default = "default_port_timeout")]
    port_timeout_ms: u64,
    #[serde(default = "default_check_interval")]
    check_interval_secs: u64,
    #[serde(default)]
    notifications: NotificationSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port_timeout_ms: default_port_timeout(),
            check_interval_secs: default_check_interval(),
            notifications: NotificationSettings::default(),
        }
    }
}

fn default_port_timeout() -> u64 { 2000 }
fn default_check_interval() -> u64 { 60 }

#[derive(Deserialize, Debug, Clone, Default)]
struct NotificationSettings {
    #[serde(default)]
    desktop: bool,
    #[serde(default)]
    webhook: Option<WebhookConfig>,
    #[serde(default)]
    log: Option<LogConfig>,
}

#[derive(Deserialize, Debug, Clone)]
struct WebhookConfig {
    url: String,
}

#[derive(Deserialize, Debug, Clone)]
struct LogConfig {
    path: String,
}

/// Representa o resultado da verificação para um servidor
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ServerStatus {
    name: String,
    host: String,
    is_online: bool,
    open_ports: Vec<u16>,
    timestamp: String,
}

#[derive(Serialize, Debug)]
struct CheckReport {
    online: usize,
    total: usize,
    servers: Vec<ServerStatus>,
}

/// Função para verificar se um host está online via ping.
/// Utiliza o comando ping do sistema operacional para portabilidade.
fn is_host_online(ip: &str) -> bool {
    // Determina o comando de ping com base no sistema operacional
    // Timeout fixo de 1 segundo é suficiente para redes locais
    let command = if cfg!(target_os = "windows") {
        ("ping", vec!["-n", "1", "-w", "1000", ip])
    } else {
        // Para Linux, macOS, etc.
        ("ping", vec!["-c", "1", "-W", "1", ip])
    };

    // Executa o comando de ping
    let status = Command::new(command.0)
        .args(command.1)
        .stdout(Stdio::null()) // Oculta a saída padrão
        .stderr(Stdio::null()) // Oculta a saída de erro
        .status();

    // Retorna true se o comando foi executado com sucesso (exit code 0)
    match status {
        Ok(exit_status) => exit_status.success(),
        Err(_) => false,
    }
}

/// Função para verificar se uma porta específica está aberta em um IP.
fn is_port_open(ip: &str, port: u16, timeout_ms: u64) -> bool {
    // Tenta resolver o endereço IP
    let Ok(mut addrs) = (ip, port).to_socket_addrs() else { return false; };

    // Pega o primeiro endereço resolvido
    if let Some(addr) = addrs.next() {
        // Tenta conectar com o timeout configurado
        TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).is_ok()
    } else {
        false
    }
}

/// Busca o arquivo de configuração nos locais padrão
fn find_config_file(custom_path: Option<String>) -> Result<String, String> {
    // Se o usuário especificou um arquivo, tenta usar ele
    if let Some(path) = custom_path {
        if std::path::Path::new(&path).exists() {
            return Ok(path);
        }
        return Err(format!("Arquivo especificado não encontrado: {}", path));
    }

    // Lista de locais padrão para procurar
    let config_names = vec!["network_checker.toml", "config.toml"];

    // 1. Diretório atual
    for name in &config_names {
        if std::path::Path::new(name).exists() {
            return Ok(name.to_string());
        }
    }

    // 2. ~/.config/network_checker/
    if let Some(config_dir) = dirs::config_dir() {
        let app_config_dir = config_dir.join("network_checker");
        for name in &config_names {
            let path = app_config_dir.join(name);
            if path.exists() {
                return Ok(path.to_string_lossy().to_string());
            }
        }
    }

    // 3. /etc/network_checker/ (apenas Linux/Unix)
    if cfg!(unix) {
        for name in &config_names {
            let path = format!("/etc/network_checker/{}", name);
            if std::path::Path::new(&path).exists() {
                return Ok(path);
            }
        }
    }

    Err("Nenhum arquivo de configuração encontrado. Procurados: ./network_checker.toml, ./config.toml, ~/.config/network_checker/config.toml, /etc/network_checker/config.toml".to_string())
}

/// Carrega o estado anterior dos servidores
fn load_state() -> HashMap<String, ServerStatus> {
    if let Some(cache_dir) = dirs::cache_dir() {
        let state_file = cache_dir.join("network_checker").join("state.json");
        if let Ok(content) = fs::read_to_string(&state_file) {
            if let Ok(state) = serde_json::from_str(&content) {
                return state;
            }
        }
    }
    HashMap::new()
}

/// Salva o estado atual dos servidores
fn save_state(state: &HashMap<String, ServerStatus>) {
    if let Some(cache_dir) = dirs::cache_dir() {
        let state_dir = cache_dir.join("network_checker");
        if let Err(e) = fs::create_dir_all(&state_dir) {
            eprintln!("Erro ao criar diretório de cache: {}", e);
            return;
        }

        let state_file = state_dir.join("state.json");
        if let Ok(json) = serde_json::to_string_pretty(state) {
            if let Err(e) = fs::write(&state_file, json) {
                eprintln!("Erro ao salvar estado: {}", e);
            }
        }
    }
}

/// Envia notificação desktop
fn send_desktop_notification(server_name: &str, is_online: bool) {
    #[cfg(not(target_os = "windows"))]
    {
        use notify_rust::Notification;
        let (summary, body) = if is_online {
            ("Servidor Online", format!("{} está agora ONLINE", server_name))
        } else {
            ("Servidor Offline", format!("{} está agora OFFLINE", server_name))
        };

        if let Err(e) = Notification::new()
            .summary(summary)
            .body(&body)
            .timeout(5000)
            .show()
        {
            eprintln!("Erro ao enviar notificação desktop: {}", e);
        }
    }
}

/// Envia notificação via webhook
fn send_webhook_notification(url: &str, status: &ServerStatus) {
    use reqwest::blocking::Client;

    let client = Client::new();
    if let Err(e) = client.post(url)
        .json(status)
        .send()
    {
        eprintln!("Erro ao enviar webhook: {}", e);
    }
}

/// Registra mudança em arquivo de log
fn log_notification(log_path: &str, server_name: &str, is_online: bool) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let status_str = if is_online { "ONLINE" } else { "OFFLINE" };
    let log_line = format!("[{}] {} está agora {}\n", timestamp, server_name, status_str);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        if let Err(e) = file.write_all(log_line.as_bytes()) {
            eprintln!("Erro ao escrever no log: {}", e);
        }
    } else {
        eprintln!("Erro ao abrir arquivo de log: {}", log_path);
    }
}

/// Verifica o status de um servidor
fn check_server(server: &Server, settings: &Settings) -> ServerStatus {
    let is_online = is_host_online(&server.host);
    let mut open_ports = Vec::new();

    if is_online && !server.ports.is_empty() {
        open_ports = server.ports
            .par_iter()
            .filter(|&&port| is_port_open(&server.host, port, settings.port_timeout_ms))
            .cloned()
            .collect();
    }

    ServerStatus {
        name: server.name.clone(),
        host: server.host.clone(),
        is_online,
        open_ports,
        timestamp: Local::now().to_rfc3339(),
    }
}

/// Processa notificações quando há mudança de estado
fn process_notifications(
    new_status: &ServerStatus,
    old_status: Option<&ServerStatus>,
    settings: &NotificationSettings,
) {
    // Verifica se houve mudança de estado
    let state_changed = match old_status {
        Some(old) => old.is_online != new_status.is_online,
        None => true, // Primeira execução
    };

    if !state_changed {
        return;
    }

    // Desktop notification
    if settings.desktop {
        send_desktop_notification(&new_status.name, new_status.is_online);
    }

    // Webhook notification
    if let Some(webhook) = &settings.webhook {
        send_webhook_notification(&webhook.url, new_status);
    }

    // Log notification
    if let Some(log) = &settings.log {
        log_notification(&log.path, &new_status.name, new_status.is_online);
    }
}

fn main() {
    // Parse os argumentos da linha de comando
    let args = Args::parse();

    // Busca o arquivo de configuração
    let config_path = match find_config_file(args.file) {
        Ok(path) => {
            if !args.json {
                println!("Usando arquivo de configuração: {}", path);
            }
            path
        }
        Err(e) => {
            if args.json {
                print_json_error(&e);
            } else {
                eprintln!("Erro: {}", e);
            }
            return;
        }
    };

    // Lê o arquivo de configuração
    let toml_content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(e) => {
            let msg = format!("Erro ao ler o arquivo '{}': {}", config_path, e);
            if args.json {
                print_json_error(&msg);
            } else {
                eprintln!("{}", msg);
            }
            return;
        }
    };

    // Parse o conteúdo TOML para a struct Config
    let config: Config = match toml::from_str(&toml_content) {
        Ok(parsed) => parsed,
        Err(e) => {
            let msg = format!("Erro ao parsear o arquivo TOML: {}", e);
            if args.json {
                print_json_error(&msg);
            } else {
                eprintln!("{}", msg);
            }
            return;
        }
    };

    if config.servers.is_empty() {
        let msg = "Erro: Nenhum servidor configurado no arquivo";
        if args.json {
            print_json_error(msg);
        } else {
            eprintln!("{}", msg);
        }
        return;
    }

    // Modo daemon
    if args.daemon {
        println!("Iniciando em modo daemon...");
        println!("Intervalo de verificação: {} segundos", config.settings.check_interval_secs);
        println!("Pressione Ctrl+C para sair\n");

        let mut state = load_state();

        loop {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            println!("[{}] Verificando servidores...", timestamp);

            let results: Vec<ServerStatus> = config.servers
                .par_iter()
                .map(|server| check_server(server, &config.settings))
                .collect();

            for new_status in results {
                // Processa notificações se houve mudança
                let old_status = state.get(&new_status.name);
                process_notifications(&new_status, old_status, &config.settings.notifications);

                // Exibe status
                display_server_status(&new_status);

                // Atualiza estado
                state.insert(new_status.name.clone(), new_status);
            }

            // Salva estado
            save_state(&state);

            println!();
            thread::sleep(Duration::from_secs(config.settings.check_interval_secs));
        }
    } else {
        if !args.json {
            println!("Iniciando verificação de rede...\n");
        }

        let results: Vec<ServerStatus> = config.servers
            .par_iter()
            .map(|server| {
                if !args.json {
                    println!("- Verificando: {}", server.name);
                }
                check_server(server, &config.settings)
            })
            .collect();

        if args.json {
            print_json_report(&results);
        } else {
            println!("\n--- Resultados ---");
            for status in results {
                display_server_status(&status);
            }
        }
    }
}

fn print_json_report(results: &[ServerStatus]) {
    let report = CheckReport {
        online: results.iter().filter(|s| s.is_online).count(),
        total: results.len(),
        servers: results.to_vec(),
    };
    match serde_json::to_string(&report) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Erro ao serializar JSON: {}", e),
    }
}

fn print_json_error(message: &str) {
    let payload = serde_json::json!({
        "error": message,
        "online": 0,
        "total": 0,
        "servers": []
    });
    println!("{}", payload);
}

/// Exibe o status de um servidor de forma formatada
fn display_server_status(status: &ServerStatus) {
    let online_status = if status.is_online {
        "\x1B[32m[ONLINE]\x1B[0m" // Verde
    } else {
        "\x1B[31m[OFFLINE]\x1B[0m" // Vermelho
    };

    print!("{:<20} ({:<15}) | Status: {}", status.name, status.host, online_status);

    if !status.open_ports.is_empty() {
        let port_list: String = status.open_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        print!(" | \x1B[32mPortas: {}\x1B[0m", port_list);
    } else if status.is_online && !status.open_ports.is_empty() {
        print!(" | Portas: Nenhuma aberta");
    }

    println!();
}

