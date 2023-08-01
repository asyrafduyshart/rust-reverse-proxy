use reqwest::Client;
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::interval;

use crate::config;

pub async fn create_whitelist_updater_task(
	client: Client,
	config: Arc<config::Configuration>,
	whitelisted_ips: Arc<Mutex<HashSet<IpAddr>>>,
) -> tokio::task::JoinHandle<()> {
	let parsed_timeout = config.ip_check_interval.parse::<u64>();
	let timeout = match parsed_timeout {
		Ok(val) => val,
		Err(_) => {
			eprintln!("Failed to parse ip_check_interval, using default value of 30");
			30
		}
	};

	tokio::task::spawn(async move {
		let mut interval = interval(Duration::from_secs(timeout));

		loop {
			interval.tick().await;

			let res = client.get(config.ip_whitelist_url.clone()).send().await;

			match res {
				Ok(response) => {
					let body = match response.text().await {
						Ok(body) => body,
						Err(e) => {
							eprintln!("Failed to parse response body: {}", e);
							continue;
						}
					};

					let ips: Vec<IpAddr> = body
						.lines()
						.map(|line| line.parse::<IpAddr>())
						.filter_map(Result::ok)
						.collect();

					println!("updated whitelist: {:?}", ips.len());

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
	})
}
