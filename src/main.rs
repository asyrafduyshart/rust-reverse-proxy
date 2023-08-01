mod config;
mod usecase;
mod utils;
use config::Configuration;
mod schedule_task;
use std::{
	collections::HashSet,
	env,
	fs::File,
	io::Read,
	net::{IpAddr, SocketAddr},
	path::Path,
	sync::{Arc, Mutex},
};
use tokio::task::{self};

use hyper::{
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
			let make_svc = make_service_fn(move |_| {
				let config = Arc::new(config.clone());
				let whitelisted_ips = Arc::clone(&whitelisted_ips);
				async move {
					Ok::<_, hyper::Error>(service_fn(move |req| {
						usecase::proxy::mirror(
							req,
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
		let whitelist_updater_task = schedule_task::create_whitelist_updater_task(
			client.clone(),
			Arc::new(config.clone()),
			Arc::clone(&whitelisted_ips),
		)
		.await;

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
