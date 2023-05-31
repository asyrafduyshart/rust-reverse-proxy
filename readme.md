# Rust Proxy Server

This is a Rust-based HTTP proxy server that can be used to forward incoming HTTP requests to other servers, serve static files, and allow for routing based on the request path. 

## Getting Started

First, clone the repository to your local machine using Git:

```bash
git clone https://github.com/asyrafduyshart/rust-reverse-proxy.git
cd rust-reverse-proxy
```

### Prerequisites

This project requires [Rust](https://www.rust-lang.org/) to be installed on your machine. If it's not installed, you can install it by following the instructions [here](https://www.rust-lang.org/tools/install).

### Configuration

The server is configured using a JSON configuration file. An example of the configuration file can be seen in `config.example`. You will need to create your own configuration file based on this example.

```json
{
	"log_level": "error",
	"http": {
		"servers": [{
			"root": "static",
			"name": "site2",
			"proxies": [{
				"proxy_pass": "https://apichallenges.herokuapp.com/mirror/request",
				"proxy_path": "/mirror",
				"retain_path": true,
                "request_headers": [{
					"Some-Header": "Oke"
				}]
			}
		],
			"listen": "3400"
		}]
	},
	"access_log": "verbose"
}
```

The configuration file is loaded based on the `CONFIG_SETTING` environment variable. If the variable is not set, the server will default to loading the `config.json` file from the root directory.

### Running the Server

To build and run the server, execute the following command:

```bash
cargo run
```

The server will start and listen on the port specified in the configuration file. If no port is specified, it will default to port 8080.

## Code Structure

The codebase consists of the following main components:

- `main.rs`: This is the entry point of the application. It loads the configuration and starts the server.
- `config.rs`: This module defines the data structures for the configuration file and the logic to load the configuration.
- `proxy.rs`: This module contains the logic to proxy HTTP requests, serve static files and handle unmatched requests.

## Contribution

Contributions are welcome! Please feel free to submit a pull request.

## License

This project is licensed under the terms of the MIT license. See the [LICENSE](LICENSE) file for details.
