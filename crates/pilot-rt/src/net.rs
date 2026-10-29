//! Milestone 24 & Milestone 43.4: TCP/UDP Sockets & Declarative HTTP Engine (`stdlib/net` and `stdlib/http`).

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};

pub fn tcp_connect(addr: &str) -> io::Result<TcpStream> {
    TcpStream::connect(addr)
}

pub fn tcp_listen(addr: &str) -> io::Result<TcpListener> {
    TcpListener::bind(addr)
}

pub fn udp_bind(addr: &str) -> io::Result<UdpSocket> {
    UdpSocket::bind(addr)
}

/// HTTP response containing status code, headers, and payload bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn ok(body: &[u8], content_type: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), content_type.to_string());
        headers.insert("Content-Length".to_string(), body.len().to_string());
        Self {
            status: 200,
            headers,
            body: body.to_vec(),
        }
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
}

/// Executes a synchronous HTTP GET request.
pub fn http_get(url: &str) -> Result<HttpResponse, String> {
    let (host, port, path) = parse_url(url)?;
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr).map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Pilot/1.1.0\r\nConnection: close\r\n\r\n",
        path, host
    );
    stream.write_all(req.as_bytes()).map_err(|e| format!("Write failed: {}", e))?;

    read_http_response(&mut stream)
}

/// Executes a synchronous HTTP POST request.
pub fn http_post(url: &str, body: &[u8], content_type: &str) -> Result<HttpResponse, String> {
    let (host, port, path) = parse_url(url)?;
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect(&addr).map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nUser-Agent: Pilot/1.1.0\r\nConnection: close\r\n\r\n",
        path, host, content_type, body.len()
    );
    stream.write_all(req.as_bytes()).map_err(|e| format!("Write headers failed: {}", e))?;
    stream.write_all(body).map_err(|e| format!("Write body failed: {}", e))?;

    read_http_response(&mut stream)
}

fn parse_url(url: &str) -> Result<(String, u16, String), String> {
    let without_scheme = if let Some(stripped) = url.strip_prefix("http://") {
        stripped
    } else if let Some(stripped) = url.strip_prefix("https://") {
        stripped
    } else {
        url
    };

    let (host_port, path) = if let Some(slash_idx) = without_scheme.find('/') {
        (&without_scheme[..slash_idx], &without_scheme[slash_idx..])
    } else {
        (without_scheme, "/")
    };

    let (host, port) = if let Some(colon_idx) = host_port.find(':') {
        let h = &host_port[..colon_idx];
        let p: u16 = host_port[colon_idx + 1..].parse().map_err(|_| "Invalid port")?;
        (h.to_string(), p)
    } else {
        (host_port.to_string(), 80)
    };

    Ok((host, port, path.to_string()))
}

fn read_http_response(stream: &mut TcpStream) -> Result<HttpResponse, String> {
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(|e| format!("Read response failed: {}", e))?;

    // Find \r\n\r\n delimiter
    let delim = b"\r\n\r\n";
    let header_end = raw
        .windows(4)
        .position(|w| w == delim)
        .ok_or_else(|| "Invalid HTTP response (no header boundary)".to_string())?;

    let header_bytes = &raw[..header_end];
    let body = raw[header_end + 4..].to_vec();

    let header_str = String::from_utf8_lossy(header_bytes);
    let mut lines = header_str.lines();
    let status_line = lines.next().ok_or_else(|| "Empty status line".to_string())?;

    let status_parts: Vec<&str> = status_line.split_whitespace().collect();
    let status = if status_parts.len() >= 2 {
        status_parts[1].parse::<u16>().unwrap_or(200)
    } else {
        200
    };

    let mut headers = HashMap::new();
    for line in lines {
        if let Some(colon_idx) = line.find(':') {
            let k = line[..colon_idx].trim().to_string();
            let v = line[colon_idx + 1..].trim().to_string();
            headers.insert(k, v);
        }
    }

    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}
