use std::{error::Error, fmt::Debug};

use config::{
    builder::AsyncState, AsyncSource, ConfigBuilder, ConfigError, FileFormat, Format, Map,
};

use async_trait::async_trait;
use futures::{select, FutureExt};
use warp::Filter;

// Example below presents sample configuration server and client.
//
// Server serves simple configuration on HTTP endpoint.
// Client consumes it using custom HTTP AsyncSource built on top of reqwest.

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    select! {
        r = run_server().fuse() => r,
        r = run_client().fuse() => r
    }
}

async fn run_server() -> Result<(), Box<dyn Error>> {
    let service = warp::path("configuration").map(|| r#"{ "value" : 123 }"#);

    println!("Running server on localhost:5001");

    warp::serve(service).bind(([127, 0, 0, 1], 5001)).await;

    Ok(())
}
//////////////////////
// Client
//////////////////////
async fn run_client() -> Result<(), Box<dyn Error>> {
    // Good enough for an example to allow server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = ConfigBuilder::<AsyncState>::default()
        .add_async_source(HttpAsyncSource {
            uri: "https://raw.githubusercontent.com/developerworks/crate_demo_config/main/resources/config.json".into(),
            format: FileFormat::Json,
        })
        .build()
        .await?;

    // println!("Config value is {}", config.get::<String>("value")?);
    println!("Config value is {:#?}", config);

    Ok(())
}

// Actual implementation of AsyncSource can be found below

#[derive(Debug)]
struct HttpAsyncSource<F: Format> {
    uri: String,
    format: F,
}

#[async_trait]
impl<F: Format + Send + Sync + Debug> AsyncSource for HttpAsyncSource<F> {
    async fn collect(&self) -> Result<Map<String, config::Value>, ConfigError> {
        let client = get_client(true);

        client
            .get(&self.uri)
            .send()
            .await
            .map_err(|e| ConfigError::Foreign(Box::new(e)))? // error conversion is possible from custom AsyncSource impls
            .text()
            .await
            .map_err(|e| ConfigError::Foreign(Box::new(e)))
            .and_then(|text| {
                self.format
                    .parse(Some(&self.uri), &text)
                    .map_err(|e| ConfigError::Foreign(e))
            })
    }
}

fn get_client(use_proxy: bool) -> reqwest::Client {
    let client_builder = reqwest::Client::builder();
    if use_proxy {
        let proxy =
            reqwest::Proxy::https("http://192.168.1.110:1087").expect("tor proxy should be there");
        client_builder
            .proxy(proxy)
            .build()
            .expect("should be able to build reqwest client")
    } else {
        client_builder
            .build()
            .expect("should be able to build reqwest client")
    }
}
