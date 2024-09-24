# fff

fff is a high-performance, asynchronous command-line tool written in Rust for making HTTP requests to URLs provided via standard input (stdin). It's designed to be fast, efficient, and highly configurable, making it ideal for tasks like web scraping, testing, and automation.


# Features
- Asynchronous I/O using Tokio for high concurrency.
- Customizable concurrency limits.
- Support for HTTP methods, headers, and request bodies.
- Proxy support.
- Response filtering based on status codes, content, and more.
- Colorized output for easy readability.
- Fast hashing using xxHash for saving responses uniquely.
- Ignore HTML or empty responses.
- Save responses matching specific criteria.

# Installation 

## Pre-built Binaries
Download the latest release for your platform from the [Releases page](https://github.com/Periecle/fff/releases).

## Build from Source
To build fff from source, ensure you have Rust and Cargo installed.


```shell
# Clone the repository
git clone https://github.com/Periecle/fff.git
cd fff

# Build the project in release mode and install it in your Path
cargo install --path .
``` 

# Usage

## Basic Usage
Supply URLs via stdin, one per line:

```shell
cat urls.txt | fff [OPTIONS]
```

## Options

```shell
Usage: fff [OPTIONS]

Request URLs provided on stdin fairly frickin' fast

Options:
  -b, --body <BODY>            Request body
  -d, --delay <DELAY>          Delay between issuing requests (ms) [default: 100]
  -H, --header <HEADER>        Add a header to the request (can be specified multiple times)
      --ignore-html            Don't save HTML files; useful when looking for non-HTML files only
      --ignore-empty           Don't save empty files
  -k, --keep-alive             Use HTTP Keep-Alive
  -m, --method <METHOD>        HTTP method to use (default: GET, or POST if body is specified) [default: GET]
  -M, --match <MATCH>          Save responses that include <string> in the body
  -o, --output <OUTPUT>        Directory to save responses in (will be created) [default: out]
  -s, --save-status <SAVE_STATUS>...
                               Save responses with given status code (can be specified multiple times)
  -S, --save                   Save all responses
  -x, --proxy <PROXY>          Use the provided HTTP proxy
  -h, --help                   Print help information
  -V, --version                Print version information
```

# Examples

## Basic Request

Make request to each URL, do not save any responses
```shell
cat urls.txt | fff
```

## Custom HTTP Method and Body

Make request to each URL using POST, with body and specific header. 
```shell
echo "http://example.com/api" | fff -m POST -b '{"key":"value"}' -H "Content-Type: application/json"
```

## Using a Proxy

Make request to each URL via specified proxy server.
```shell
cat urls.txt | fff -x http://proxyserver:8080
```

## Saving Responses with Specific Status Codes

Make request to each URL and save requests with status codes 200 and 300 into default directory "roots"
```shell
cat urls.txt | fff -s 200 -s 301
```

## Ignoring HTML Responses
Can be useful if you want to fetch all non-html requests.
```shell
cat urls.txt | fff --ignore-html
```

## Matching Content in Responses
Matches only content that contains specified string.

```shell
cat urls.txt | fff -M "Welcome to"
```

## Setting Concurrency and Delay

For targets that have some rate-limits, or just sensitive to high amount of requests you can setup delay between requests in milliseconds.
```shell
cat urls.txt | fff -c 50 -d 500
```

# Original Work
This tool was originally written by [tomnomnom in Go](https://github.com/tomnomnom/fff). 

Differences from the Original Go Tool
- Language: fff is written in Rust, leveraging Rust's safety and performance benefits.
- Asynchronous I/O: Utilizes Tokio for efficient asynchronous operations.
- Performance: Optimized for speed with features like ultrafast hashing using xxHash.
- Extensibility: Easier to extend and maintain due to Rust's powerful type system and package ecosystem.
- Enhanced Features: Additional options like ignoring HTML content, matching response bodies, and colorized output.
- Dependency Management: Uses Cargo for dependency management, simplifying the build process.

# Contributing
Contributions are welcome! Please open an issue or submit a pull request on GitHub.

# Fork the repository.
Create a new branch with your feature or bug fix.
Commit your changes with clear messages.
Push to your branch and open a pull request.
Ensure that all tests pass and adhere to the existing code style.

# License
This project is licensed under the MIT License.

