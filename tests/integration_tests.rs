// tests/integration_tests.rs

use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use regex::Regex;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

// Include the normalise_path function to match the application's logic
fn normalise_path(url: &reqwest::Url) -> String {
    let path = url.path();
    let re = Regex::new(r"[^a-zA-Z0-9/._-]+").unwrap();
    let normalised = re.replace_all(path, "-").to_string();
    // Remove leading slashes to ensure the path is relative
    let normalised = normalised.trim_start_matches('/').to_string();
    // If the path is empty after trimming, use a default name
    if normalised.is_empty() {
        "root".to_string()
    } else {
        normalised
    }
}

#[tokio::test]
async fn test_basic_request() {
    // Start a mock server
    let server = MockServer::start_async().await;

    // Create a mock response
    let body = "Hello, world!";

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200)
            .header("Content-Type", "text/plain")
            .body(body);
    });

    // Use a temporary output directory
    let temp_dir = TempDir::new().unwrap();

    {
        // Prepare the command
        let mut cmd = Command::cargo_bin("fff").unwrap();

        // Set the arguments
        cmd.arg("-o").arg(temp_dir.path()).arg("-S"); // Save all responses

        // Provide the URL via stdin
        cmd.write_stdin(format!("{}\n", server.url("/")));

        // Run the command and capture output
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Saved"));
    }

    // Verify that the response body file was created
    let host = server.address().ip().to_string();

    // Compute the normalized path
    let url = reqwest::Url::parse(&server.url("/")).unwrap();
    let normalised_path = normalise_path(&url);

    let expected_dir = temp_dir.path().join(host).join(normalised_path);
    let entries = fs::read_dir(&expected_dir).expect("Expected directory not found");
    let mut found_body = false;
    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("body") {
            let content = fs::read_to_string(&path).expect("Failed to read body file");
            assert_eq!(content, body);
            found_body = true;
            break;
        }
    }
    assert!(found_body, "Response body file not found");
}

#[tokio::test]
async fn test_post_request_with_body() {
    // Start a mock server
    let server = MockServer::start_async().await;

    // Create a mock response
    let body = "Post response";

    let _mock = server.mock(|when, then| {
        when.method(POST).path("/post").body("test data");
        then.status(200).body(body);
    });

    // Use a temporary output directory
    let temp_dir = TempDir::new().unwrap();

    {
        // Prepare the command
        let mut cmd = Command::cargo_bin("fff").unwrap();

        // Set the arguments
        cmd.arg("-o")
            .arg(temp_dir.path())
            .arg("-b")
            .arg("test data")
            .arg("-S");

        // Provide the URL via stdin
        cmd.write_stdin(format!("{}\n", server.url("/post")));

        // Run the command and capture output
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Saved"));
    }

    // Verify that the response body file was created
    let host = server.address().ip().to_string();

    // Compute the normalized path
    let url = reqwest::Url::parse(&server.url("/post")).unwrap();
    let normalised_path = normalise_path(&url);

    let expected_dir = temp_dir.path().join(host).join(normalised_path);
    let entries = fs::read_dir(&expected_dir).expect("Expected directory not found");
    let mut found_body = false;
    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("body") {
            let content = fs::read_to_string(&path).expect("Failed to read body file");
            assert_eq!(content, body);
            found_body = true;
            break;
        }
    }
    assert!(found_body, "Response body file not found");
}

#[tokio::test]
async fn test_match_option() {
    // Start a mock server
    let server = MockServer::start_async().await;

    // Create a mock response containing "special string"
    let body = "This response contains a special string.";

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200).body(body);
    });

    // Use a temporary output directory
    let temp_dir = TempDir::new().unwrap();

    {
        // Prepare the command
        let mut cmd = Command::cargo_bin("fff").unwrap();

        // Set the arguments
        cmd.arg("-o")
            .arg(temp_dir.path())
            .arg("-M")
            .arg("special string");

        // Provide the URL via stdin
        cmd.write_stdin(format!("{}\n", server.url("/")));

        // Run the command and capture output
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Saved"));
    }

    // Verify that the response body file was created
    let host = server.address().ip().to_string();

    // Compute the normalized path
    let url = reqwest::Url::parse(&server.url("/")).unwrap();
    let normalised_path = normalise_path(&url);

    let expected_dir = temp_dir.path().join(host).join(normalised_path);
    let entries = fs::read_dir(&expected_dir).expect("Expected directory not found");
    let mut found_body = false;
    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("body") {
            let content = fs::read_to_string(&path).expect("Failed to read body file");
            assert_eq!(content, body);
            found_body = true;
            break;
        }
    }
    assert!(found_body, "Response body file not found");
}

#[tokio::test]
async fn test_save_status() {
    // Start a mock server
    let server = MockServer::start_async().await;

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(404).body("Not Found");
    });

    // Use a temporary output directory
    let temp_dir = TempDir::new().unwrap();

    {
        // Prepare the command
        let mut cmd = Command::cargo_bin("fff").unwrap();

        // Set the arguments
        cmd.arg("-o").arg(temp_dir.path()).arg("-s").arg("404"); // Save responses with status 404

        // Provide the URL via stdin
        cmd.write_stdin(format!("{}\n", server.url("/")));

        // Run the command and capture output
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Saved"));
    }

    // Verify that the response body file was created
    let host = server.address().ip().to_string();

    // Compute the normalized path
    let url = reqwest::Url::parse(&server.url("/")).unwrap();
    let normalised_path = normalise_path(&url);

    let expected_dir = temp_dir.path().join(host).join(normalised_path);
    let entries = fs::read_dir(&expected_dir).expect("Expected directory not found");
    let mut found_body = false;
    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("body") {
            let content = fs::read_to_string(&path).expect("Failed to read body file");
            assert_eq!(content, "Not Found");
            found_body = true;
            break;
        }
    }
    assert!(found_body, "Response body file not found");
}

#[tokio::test]
async fn test_delay_between_requests() {
    // Start a mock server
    let server = MockServer::start_async().await;

    // Create mock responses
    let _mock1 = server.mock(|when, then| {
        when.method(GET).path("/1");
        then.status(200);
    });

    let _mock2 = server.mock(|when, then| {
        when.method(GET).path("/2");
        then.status(200);
    });

    // Use a temporary output directory
    let temp_dir = TempDir::new().unwrap();

    // Prepare the command
    let mut cmd = Command::cargo_bin("fff").unwrap();

    // Set delay to 500ms
    cmd.arg("-d")
        .arg("500")
        .arg("-S") // Save all responses
        .arg("-o")
        .arg(temp_dir.path()); // Output directory

    // Provide the URLs via stdin
    cmd.write_stdin(format!("{}\n{}\n", server.url("/1"), server.url("/2")));

    // Record the time before running
    let start_time = std::time::Instant::now();

    // Run the command and capture output
    cmd.assert().success();

    // Check that the time taken is at least 500ms
    let elapsed = start_time.elapsed();
    assert!(
        elapsed >= Duration::from_millis(500),
        "Elapsed time was less than 500ms"
    );
}

#[tokio::test]
async fn test_custom_headers() {
    // Start a mock server
    let server = MockServer::start_async().await;

    // Create a mock response
    let _mock = server.mock(|when, then| {
        when.method(GET)
            .path("/")
            .header("X-Test-Header", "HeaderValue");
        then.status(200);
    });

    // Prepare the command
    let mut cmd = Command::cargo_bin("fff").unwrap();

    // Set header
    cmd.arg("-H").arg("X-Test-Header: HeaderValue");

    // Provide the URL via stdin
    cmd.write_stdin(format!("{}\n", server.url("/")));

    // Run the command and capture output
    cmd.assert().success();

    // Verify that the mock was called
    let hits = _mock.hits();
    assert_eq!(
        hits, 1,
        "The mock server did not receive the expected header"
    );
}

#[tokio::test]
async fn test_ignore_html() {
    // Start a mock server
    let server = MockServer::start_async().await;

    // Create a mock HTML response
    let html_body = "<html><body>Test</body></html>";

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200)
            .header("Content-Type", "text/html")
            .body(html_body);
    });

    // Use a temporary output directory
    let temp_dir = TempDir::new().unwrap();

    {
        // Prepare the command
        let mut cmd = Command::cargo_bin("fff").unwrap();

        // Set arguments
        cmd.arg("-o")
            .arg(temp_dir.path())
            .arg("--ignore-html")
            .arg("-S"); // Save all responses

        // Provide the URL via stdin
        cmd.write_stdin(format!("{}\n", server.url("/")));

        // Run the command and capture output
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("200"));
    }

    // Verify that the response body file was not created
    let host = server.address().ip().to_string();

    // Compute the normalized path
    let url = reqwest::Url::parse(&server.url("/")).unwrap();
    let normalised_path = normalise_path(&url);

    let expected_dir = temp_dir.path().join(host).join(normalised_path);
    let entries = fs::read_dir(&expected_dir);
    assert!(
        entries.is_err() || entries.unwrap().next().is_none(),
        "Response body file should not be saved"
    );
}

#[tokio::test]
async fn test_ignore_empty() {
    // Start a mock server
    let server = MockServer::start_async().await;

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200).body("");
    });

    // Use a temporary output directory
    let temp_dir = TempDir::new().unwrap();

    {
        // Prepare the command
        let mut cmd = Command::cargo_bin("fff").unwrap();

        // Set arguments
        cmd.arg("-o")
            .arg(temp_dir.path())
            .arg("--ignore-empty")
            .arg("-S"); // Save all responses

        // Provide the URL via stdin
        cmd.write_stdin(format!("{}\n", server.url("/")));

        // Run the command and capture output
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("200"));
    }

    // Verify that the response body file was not created
    let host = server.address().ip().to_string();

    // Compute the normalized path
    let url = reqwest::Url::parse(&server.url("/")).unwrap();
    let normalised_path = normalise_path(&url);

    let expected_dir = temp_dir.path().join(host).join(normalised_path);
    let entries = fs::read_dir(&expected_dir);
    assert!(
        entries.is_err() || entries.unwrap().next().is_none(),
        "Response body file should not be saved"
    );
}

#[tokio::test]
async fn test_proxy_option() {
    // Start a mock server to act as the proxy
    let proxy_server = MockServer::start_async().await;

    // Simulate a proxy by allowing any method and path
    let _proxy_mock = proxy_server.mock(|when, then| {
        when.any_request();
        then.status(200);
    });

    // Start another mock server to handle the actual request
    let server = MockServer::start_async().await;

    // Create a mock response
    let _mock = server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200).body("Proxied response");
    });

    // Prepare the command
    let mut cmd = Command::cargo_bin("fff").unwrap();

    // Set the proxy
    cmd.arg("-x")
        .arg(format!("http://{}", proxy_server.address()));

    // Provide the URL via stdin
    cmd.write_stdin(format!("{}\n", server.url("/")));

    // Run the command and capture output
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("200"));

    // Verify that the proxy server received the request
    let hits = _proxy_mock.hits();
    assert!(hits > 0, "Proxy server was not used");
}
