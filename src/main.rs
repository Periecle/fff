use bytes::Bytes;
use clap::Parser;
use colored::Colorize;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method, Proxy, StatusCode, Url, Version};
use std::io::{self};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs as tokio_fs;
use tokio::io::{self as tokio_io, AsyncBufReadExt};
use tokio::sync::Semaphore;
use tokio::time::sleep;
use xxhash_rust::xxh3::Xxh3; // Import bytes::Bytes

/// Command-line arguments structure using `clap`
#[derive(Parser, Debug, Clone)]
#[command(
    name = "fff",
    about = "Request URLs provided on stdin fairly frickin' fast",
    version = "1.0"
)]
struct Opts {
    /// Request body
    #[arg(short = 'b', long)]
    body: Option<String>,

    /// Delay between issuing requests (ms)
    #[arg(short = 'd', long, default_value_t = 100)]
    delay: u64,

    /// Add a header to the request (can be specified multiple times)
    #[arg(short = 'H', long)]
    header: Vec<String>,

    /// Don't save HTML files; useful when looking for non-HTML files only
    #[arg(long = "ignore-html")]
    ignore_html: bool,

    /// Don't save empty files
    #[arg(long = "ignore-empty")]
    ignore_empty: bool,

    /// Use HTTP Keep-Alive
    #[arg(short = 'k', long = "keep-alive", alias = "keep-alives")]
    keep_alive: bool,

    /// HTTP method to use (default: GET, or POST if body is specified)
    #[arg(short = 'm', long, default_value = "GET")]
    method: String,

    /// Save responses that include <string> in the body
    #[arg(short = 'M', long)]
    r#match: Option<String>,

    /// Directory to save responses in (will be created)
    #[arg(short = 'o', long, default_value = "out")]
    output: PathBuf,

    /// Save responses with given status code (can be specified multiple times)
    #[arg(short = 's', long = "save-status")]
    save_status: Vec<u16>,

    /// Save all responses
    #[arg(short = 'S', long = "save")]
    save: bool,

    /// Use the provided HTTP proxy
    #[arg(short = 'x', long = "proxy")]
    proxy: Option<String>,
}

// Define the ResponseData struct to encapsulate response-related data
struct ResponseData {
    method: Method,
    raw_url: String,
    response_body: Bytes,
    resp_headers: HeaderMap,
    resp_url: Url,
    status: StatusCode,
    version: Version,
}

#[tokio::main]
async fn main() {
    let opts = Arc::new(Opts::parse());
    let client = match new_client(&opts) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("{}", format!("Failed to create HTTP client: {}", e).red());
            std::process::exit(1);
        }
    };

    let semaphore = Arc::new(Semaphore::new(100)); // Limit concurrency to 100
    let mut tasks = FuturesUnordered::new();

    let stdin = tokio_io::stdin();
    let reader = tokio_io::BufReader::new(stdin);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await.unwrap_or_else(|e| {
        eprintln!("{}", format!("Error reading line from stdin: {}", e).red());
        None
    }) {
        let url = line;
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = Arc::clone(&client);
        let opts = Arc::clone(&opts);

        tasks.push(tokio::spawn(async move {
            if opts.delay > 0 {
                sleep(Duration::from_millis(opts.delay)).await;
            }
            process_url(client, opts, url).await;
            drop(permit);
        }));

        while tasks.len() >= 100 {
            tasks.next().await;
        }
    }

    while tasks.next().await.is_some() {}
}

fn new_client(opts: &Opts) -> Result<Client, reqwest::Error> {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true);

    if !opts.keep_alive {
        builder = builder.pool_idle_timeout(Duration::from_secs(0));
    }

    if let Some(ref proxy_url) = opts.proxy {
        builder = builder.proxy(Proxy::all(proxy_url)?);
    }

    builder.build()
}

async fn process_url(client: Arc<Client>, opts: Arc<Opts>, raw_url: String) {
    let mut method = opts.method.clone();
    let request_body = opts.body.clone();

    if request_body.is_some() && method.eq_ignore_ascii_case("GET") {
        method = "POST".to_string();
    }

    let url = match Url::parse(&raw_url) {
        Ok(u) => u,
        Err(_) => {
            eprintln!("{}", format!("Invalid URL: {}", raw_url).red());
            return;
        }
    };

    let method = method.parse::<Method>().unwrap_or(Method::GET);

    let mut req = client.request(method.clone(), url.clone());

    // Add headers
    if let Some(headers) = parse_headers(&opts.header) {
        req = req.headers(headers);
    }

    // Add body
    if let Some(body) = request_body.clone() {
        req = req.body(body);
    }

    // Send the request
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", format!("Request failed for {}: {}", raw_url, e).red());
            return;
        }
    };

    // Extract response data
    let status = resp.status();
    let version = resp.version();
    let resp_headers = resp.headers().clone();
    let resp_url = resp.url().clone();
    let response_body = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "{}",
                format!("Failed to read body for {}: {}", raw_url, e).red()
            );
            return;
        }
    };

    // Create ResponseData instance
    let response_data = ResponseData {
        method: method.clone(),
        raw_url: raw_url.clone(),
        response_body,
        resp_headers,
        resp_url,
        status,
        version,
    };

    let mut should_save =
        opts.save || (!opts.save_status.is_empty() && opts.save_status.contains(&status.as_u16()));

    // Check if response is HTML
    if opts.ignore_html && is_html(&response_data.response_body) {
        should_save = false;
    }

    // Check if response body is empty or whitespace
    if opts.ignore_empty
        && response_data
            .response_body
            .iter()
            .all(|&b| b.is_ascii_whitespace())
    {
        should_save = false;
    }

    // Check if response body contains the match string
    if let Some(ref m) = opts.r#match {
        should_save = twoway::find_bytes(&response_data.response_body, m.as_bytes()).is_some();
    }

    if !should_save {
        println!("{} {}", raw_url, colorize_status(status));
        return;
    }

    if let Err(e) = save_response(&opts, &response_data).await {
        eprintln!(
            "{}",
            format!("Failed to save response for {}: {}", raw_url, e).red()
        );
    } else {
        println!(
            "{} {}",
            raw_url,
            format!("Saved ({})", status.as_u16()).green()
        );
    }
}

/// Function to colorize HTTP status codes
fn colorize_status(status: StatusCode) -> colored::ColoredString {
    let status_code = status.as_u16();
    let status_str = status.as_str();

    match status_code {
        200..=299 => status_str.green(),
        300..=399 => status_str.cyan(),
        400..=499 => status_str.yellow(),
        500..=599 => status_str.red(),
        _ => status_str.normal(),
    }
}

fn parse_headers(headers: &[String]) -> Option<HeaderMap> {
    let mut header_map = HeaderMap::new();
    for h in headers {
        if let Some((name, value)) = h.split_once(':') {
            let name = name.trim();
            let value = value.trim();
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                header_map.append(name, value);
            }
        }
    }
    if header_map.is_empty() {
        None
    } else {
        Some(header_map)
    }
}

fn is_html(body: &[u8]) -> bool {
    body.windows(5).any(|w| w.eq_ignore_ascii_case(b"<html"))
}

async fn save_response(opts: &Opts, response_data: &ResponseData) -> io::Result<()> {
    let method = &response_data.method;
    let raw_url = &response_data.raw_url;
    let response_body = &response_data.response_body;
    let resp_headers = &response_data.resp_headers;
    let resp_url = &response_data.resp_url;
    let status = response_data.status;
    let version = response_data.version;

    let normalised_path = normalise_path(resp_url);

    let hash_input = format!(
        "{}{}{}{}",
        method,
        raw_url,
        opts.body.clone().unwrap_or_default(),
        opts.header.join("")
    );

    // Use xxHash instead of SHA1
    let mut hasher = Xxh3::new();
    hasher.update(hash_input.as_bytes());
    let hash = hasher.digest();
    let hash_hex = format!("{:016x}", hash);

    let host = resp_url.host_str().unwrap_or("unknown");
    let output_dir = opts.output.join(host).join(normalised_path);

    tokio_fs::create_dir_all(&output_dir).await?;

    let body_filename = output_dir.join(format!("{}.body", hash_hex));
    tokio_fs::write(&body_filename, response_body).await?;

    let headers_filename = output_dir.join(format!("{}.headers", hash_hex));
    let mut buf = String::with_capacity(1024);

    // Request line
    buf.push_str(&format!("{} {}\n\n", method, raw_url));

    // Request headers
    for h in &opts.header {
        buf.push_str(&format!("> {}\n", h));
    }
    buf.push('\n');

    // Request body
    if let Some(body) = &opts.body {
        buf.push_str(body);
        buf.push_str("\n\n");
    }

    // Status line
    let version_str = match version {
        Version::HTTP_09 => "0.9",
        Version::HTTP_10 => "1.0",
        Version::HTTP_11 => "1.1",
        Version::HTTP_2 => "2",
        Version::HTTP_3 => "3",
        _ => "unknown",
    };

    buf.push_str(&format!(
        "< HTTP/{} {} {}\n",
        version_str,
        status.as_u16(),
        status.canonical_reason().unwrap_or("")
    ));

    // Response headers
    for (k, v) in resp_headers.iter() {
        buf.push_str(&format!("< {}: {}\n", k, v.to_str().unwrap_or("")));
    }

    tokio_fs::write(&headers_filename, buf).await?;

    Ok(())
}

static PATH_NORMALISE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9/._-]+").unwrap());

fn normalise_path(url: &Url) -> String {
    let path = url.path();
    let normalised = PATH_NORMALISE_RE.replace_all(path, "-").to_string();
    let normalised = normalised.trim_start_matches('/').to_string();
    if normalised.is_empty() {
        "root".to_string()
    } else {
        normalised
    }
}
