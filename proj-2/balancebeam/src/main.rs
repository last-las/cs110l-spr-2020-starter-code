mod request;
mod response;

use clap::{App, Arg};
use rand::{Rng, SeedableRng};
use tokio::net::{TcpListener, TcpStream};
use std::sync::{Arc};
use tokio::sync::RwLock;
use tokio::time::{delay_for, Duration};
use std::collections::HashMap;

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Debug)]
struct CmdOptions {
    bind: String,
    upstream: Vec<String>,
    active_health_check_interval: usize,
    active_health_check_path: String,
    max_requests_per_minute: usize,
}

fn construct_cmd_options() -> CmdOptions {
    let matches = App::new("Fun with load balancing")
        .arg(Arg::new("bind")
            .short('b')
            .long("bind")
            .about("IP/port to bind to")
            .default_value("0.0.0.0:1100"))
        .arg(Arg::new("upstream")
            .short('u')
            .long("upstream")
            .about("Upstream host to forward requests to")
            .takes_value(true)
            .multiple_values(true)
            .multiple_occurrences(true)
            .required(true))
        .arg(Arg::new("active-health-check-interval")
            .long("active-health-check-interval")
            .about("Perform active health checks on this interval (in seconds)")
            .default_value("10"))
        .arg(Arg::new("active-health-check-path")
            .long("active-health-check-path")
            .about("Path to send request to for active health checks")
            .default_value("/"))
        .arg(Arg::new("max-requests-per-minute")
            .long("max-requests-per-minute")
            .about("Maximum number of requests to accept per IP per minute(0 = unlimited)")
            .default_value("0"))
        .get_matches();

    let bind = matches.value_of("bind").unwrap().to_string();
    let upstream = matches.values_of_lossy("upstream").unwrap();
    let active1 = matches.value_of("active-health-check-interval")
        .unwrap().parse::<usize>().unwrap();
    let active2 = matches.value_of("active-health-check-path").unwrap().to_string();
    let max_requests_per_m = matches.value_of("max-requests-per-minute")
        .unwrap().parse::<usize>().unwrap();

    CmdOptions {
        bind,
        upstream,
        active_health_check_interval: active1,
        active_health_check_path: active2,
        max_requests_per_minute: max_requests_per_m,
    }
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Addresses of servers that we are proxying to
    upstream_addresses: Vec<String>,
    /// Addresses of servers that we are proxying to but failed right now
    failed_upstream_addresses: Vec<String>,
    /// requests count for each ip address per minute
    requests_cnt_map: HashMap<String, usize>
}

#[tokio::main]
async fn main() {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = construct_cmd_options();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let mut listener = match TcpListener::bind(&options.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);

    // Handle incoming connections
    let state = ProxyState {
        upstream_addresses: options.upstream,
        active_health_check_interval: options.active_health_check_interval,
        active_health_check_path: options.active_health_check_path,
        max_requests_per_minute: options.max_requests_per_minute,
        failed_upstream_addresses: Vec::new(),
        requests_cnt_map: HashMap::new()
    };

    let mutex_state = Arc::new(RwLock::new(state));
    let state_copy = mutex_state.clone();

    tokio::spawn(async {
        upstreams_active_health_check(state_copy).await;
    });

    // task which clear the counter for each address per minute
    let state_copy = mutex_state.clone();
    tokio::spawn(
        clear_counter(state_copy)
    );

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let state_copy = mutex_state.clone();
        tokio::spawn(async move {
            handle_connection(stream, state_copy).await;
        });
    };
}

async fn upstreams_active_health_check(lock_state: Arc<RwLock<ProxyState>>) {
    let active_health_check_interval;
    let active_health_check_path;
    {
        let state = lock_state.read().await;
        active_health_check_interval = state.active_health_check_interval as u64;
        active_health_check_path = state.active_health_check_path.clone();
    }

    loop {
        delay_for(Duration::from_secs(active_health_check_interval)).await;

        let mut flag: bool = false;
        let mut active_upstream_addresses = Vec::new();
        let mut passive_upstream_addresses = Vec::new();
        {
            let state = lock_state.read().await;
            let len1 = state.upstream_addresses.len();
            let len2 = state.failed_upstream_addresses.len();
            for i in 0..len1 + len2 {
                let cur_address;

                if i >= len1 {
                    cur_address = &state.failed_upstream_addresses[i - len1];
                } else {
                    cur_address = &state.upstream_addresses[i];
                }
                match perform_check_on_address(cur_address, &active_health_check_path).await {
                    Err(_) => {
                        if i < len1 {
                            flag = true;
                        }
                        passive_upstream_addresses.push(
                            String::from(cur_address)
                        );
                    }
                    Ok(_) => {
                        if i >= len1 {
                            flag = true;
                        }
                        active_upstream_addresses.push(
                            String::from(cur_address)
                        );
                    }
                }
            }
        }

        if flag {
            let mut state = lock_state.write().await;
            state.upstream_addresses = active_upstream_addresses;
            state.failed_upstream_addresses = passive_upstream_addresses;
        }
    }
}

async fn perform_check_on_address(address: &str, path: &str) -> Result<(), &'static str> {
    // 1. build an upstream
    match TcpStream::connect(address).await {
        Ok( mut upstream) => {
            let request = http::Request::builder()
                .method(http::Method::GET)
                .uri(path)
                .header("Host", address)
                .body(Vec::new())
                .unwrap();

            request::write_to_stream(&request, &mut upstream).await.unwrap();
            let result = response::read_from_stream(
                &mut upstream, request.method()).await;
            if result.is_ok() {
                let response = response::format_response_line(&result.unwrap());
                if response.contains("HTTP/1.1 200 OK") {
                    Ok(())
                }else {
                    Err("Upstream didn't return 200 status code.")
                }
            }else {
                Err("")
            }
        },
        Err(_) => {
            Err("Cannot create socket.")
        }
    }
}

async fn clear_counter(lock_state: Arc<RwLock<ProxyState>>) {
    loop {
        delay_for(Duration::from_secs(60)).await;
        let mut state = lock_state.write().await;
        state.requests_cnt_map.clear();
    }
}

async fn connect_to_upstream(mutex_state: Arc<RwLock<ProxyState>>) -> Result<TcpStream, &'static str> {
    loop {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let upstream_idx: usize;
        let upstream_ip: String;
        {
            let state = mutex_state.read().await;

            if state.upstream_addresses.len() == 0 {
                return Err("No available upstreams right now.");
            }
            upstream_idx = rng.gen_range(0, state.upstream_addresses.len());
            upstream_ip = String::from(&state.upstream_addresses[upstream_idx]);
        }

        let result = TcpStream::connect(&upstream_ip).await;

        if result.is_ok() {
            return Ok(result.unwrap());
        } else {
            let mut state = mutex_state.write().await;
            state.upstream_addresses.remove(upstream_idx);
            state.failed_upstream_addresses.push(upstream_ip);
        }
    }
}

async fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("{} <- {}", client_ip, response::format_response_line(&response));
    if let Err(error) = response::write_to_stream(&response, client_conn).await {
        log::warn!("Failed to send response to client: {}", error);
        return;
    }
}

async fn handle_connection(mut client_conn: TcpStream, state: Arc<RwLock<ProxyState>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);

    // Open a connection to a random destination server
    let mut upstream_conn = match connect_to_upstream(state.clone()).await {
        Ok(stream) => stream,
        Err(_error) => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
    };
    let upstream_ip = client_conn.peer_addr().unwrap().ip().to_string();

    // The client may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // Read a request from the client
        let request = request::read_from_stream(&mut client_conn).await;

        let mut request = match request{
            Ok(request) => {
                // Check the windows size for client_ip
                let copy_state = state.clone();
                if !check_fixed_windows_for_ip(&client_ip,copy_state).await {
                    let response = response::make_http_error(http::StatusCode::TOO_MANY_REQUESTS);
                    send_response(&mut client_conn, &response).await;
                    return;
                }

                request
            },
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response).await;
                continue;
            }
        };
        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );

        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::error!("Failed to send request to upstream {}: {}", upstream_ip, error);
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response).await;
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response).await;
        log::debug!("Forwarded response to client");
    }
}

async fn check_fixed_windows_for_ip(client_ip: &str, lock_state: Arc<RwLock<ProxyState>>) -> bool {
    let mut state = lock_state.write().await;
    // if max_requests_per_minute is zero then always return true
    if state.max_requests_per_minute == 0 {
        return true;
    }

    let cnt = state.requests_cnt_map.get(client_ip);
    match cnt {
        Some(cnt) => {
            println!("{:?}", state.requests_cnt_map);
            let val = cnt.clone() + 1;
            if val > state.max_requests_per_minute {
                return false;
            }else {
                state.requests_cnt_map.insert(client_ip.to_string(), val);
                return true;
            }
        },
        None => {
            state.requests_cnt_map.insert(client_ip.to_string(), 1);
            return true;
        }
    }
}
