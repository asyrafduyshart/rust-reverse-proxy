mod config;
mod proxy;
mod utils;
use config::Configuration;
use std::{
	collections::HashSet,
	env,
	fs::File,
	io::Read,
	net::{IpAddr, SocketAddr},
	path::Path,
	sync::{Arc, Mutex},
	time::Duration,
};
use tokio::{
	task::{self},
	time::interval,
};

use hyper::{
	server::conn::AddrStream,
	service::{make_service_fn, service_fn},
	Server,
};

// Use Jemalloc only for musl-64 bits platforms
#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() {
	// Get the CONFIG_SETTING environment variable
	let config_setting = env::var("CONFIG_SETTING");

	let config: Configuration;

	match config_setting {
		Ok(val) => {
			// If CONFIG_SETTING is set, parse it
			config = serde_json::from_str(&val).expect("JSON was not well-formatted");
		}
		Err(_) => {
			// If CONFIG_SETTING is not set, parse the file
			let json_file_path = Path::new("config.json");
			let mut json_file = File::open(&json_file_path).expect("File open failed");
			let mut json_content = String::new();
			json_file
				.read_to_string(&mut json_content)
				.expect("File read failed");
			config = serde_json::from_str(&json_content).expect("JSON was not well-formatted");
		}
	}

	let mut server_tasks = Vec::new();

	let whitelisted_ips: Arc<Mutex<HashSet<IpAddr>>> = Arc::new(Mutex::new(HashSet::new()));
	let client = reqwest::Client::new();

	for server in &config.http.servers {
		let config_port = server.listen.clone();
		let config = Arc::new(config.clone());
		let whitelisted_ips = Arc::clone(&whitelisted_ips);

		let server_task = task::spawn(async move {
			let make_svc = make_service_fn(move |socket: &AddrStream| {
				let config = Arc::new(config.clone());
				let whitelisted_ips = Arc::clone(&whitelisted_ips);
				let remote_addr = socket.remote_addr().ip();
				async move {
					Ok::<_, hyper::Error>(service_fn(move |req| {
						proxy::mirror(
							req,
							remote_addr.clone(),
							Arc::clone(&whitelisted_ips),
							Arc::clone(&config),
						)
					}))
				}
			});

			// get port from env to int or get from env
			let port = config_port.parse::<u16>().unwrap_or_else(|_| {
				env::var("PORT")
					.unwrap_or_else(|_| "8080".to_string())
					.parse::<u16>()
					.expect("PORT must be a number")
			});

			let addr = SocketAddr::from(([0, 0, 0, 0], port));
			let server = Server::bind(&addr).serve(make_svc);

			if let Err(e) = server.await {
				println!("error: {}", e);
			}
		});
		server_tasks.push(server_task);
	}

	// if whitelist url is not empty
	if !config.ip_whitelist_url.is_empty() {
		let parsed_timeout = config.ip_check_interval.parse::<u64>();
		let timeout = match parsed_timeout {
			Ok(val) => val,
			Err(_) => {
				eprintln!("Failed to parse ip_check_interval, using default value of 30");
				30
			}
		};
		let whitelist_updater_task = task::spawn(async move {
			// Create an interval timer that ticks every 30 seconds
			let mut interval = interval(Duration::from_secs(timeout));

			loop {
				// Wait for the next tick
				interval.tick().await;

				// Send the GET request
				let res = client.get(config.ip_whitelist_url.clone()).send().await;

				match res {
					Ok(response) => {
						// If the request was successful, parse the response body as a list of IPs
						let body = match response.text().await {
							Ok(body) => body,
							Err(e) => {
								eprintln!("Failed to parse response body: {}", e);
								continue;
							}
						};

						// Split the body by newline and parse each piece into an IP address
						let ips: Vec<IpAddr> = body
							.lines()
							.map(|line| line.parse::<IpAddr>())
							.filter_map(Result::ok)
							.collect();

						println!("updated whitelist: {:?}", ips.len());

						// Lock the whitelist and update it
						let mut whitelist = whitelisted_ips.lock().unwrap();
						whitelist.clear();
						for ip in ips {
							whitelist.insert(ip);
						}
					}
					Err(e) => {
						eprintln!("Failed to send GET request: {}", e);
					}
				}
			}
		});
		server_tasks.push(whitelist_updater_task);
	}

	let servers = futures::future::join_all(server_tasks).await;

	for server in servers {
		match server {
			Ok(_) => println!("Server finished successfully"),
			Err(e) => eprintln!("Server failed with error: {}", e),
		}
	}
	// clone port value from config
}
