mod config;
mod proxy;
use std::{net::SocketAddr, path::Path, fs::File, env, io::Read, sync::Arc};
use config::Configuration;

use hyper::{service::{service_fn, make_service_fn}, Server};


#[tokio::main]
async fn main() {
    // Get the CONFIG_SETTING environment variable
    let config_setting = env::var("CONFIG_SETTING");

    let config: Configuration;

    match config_setting {
        Ok(val) => {
            // If CONFIG_SETTING is set, parse it
            config = serde_json::from_str(&val).expect("JSON was not well-formatted");
        },
        Err(_) => {
            // If CONFIG_SETTING is not set, parse the file
            let json_file_path = Path::new("config.json");
            let mut json_file = File::open(&json_file_path).expect("File open failed");
            let mut json_content = String::new();
            json_file.read_to_string(&mut json_content).expect("File read failed");
            config = serde_json::from_str(&json_content).expect("JSON was not well-formatted");
        }
    }

    let make_svc = make_service_fn(move |_| {
        let config = Arc::new(config.clone());
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| proxy::mirror(req, Arc::clone(&config))))
        }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 3400));
    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        println!("error: {}", e);
    }
}