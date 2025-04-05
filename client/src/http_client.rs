use std::error::Error;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug)]
pub struct HttpClient {
    host: String,
    port: u16,
    connect_timeout: Duration,
    read_write_timeout: Duration,
}

impl HttpClient {
    pub fn new(
        host: String,
        port: u16,
        connect_timeout: Duration,
        read_write_timeout: Duration,
    ) -> Self {
        HttpClient {
            host,
            port,
            connect_timeout,
            read_write_timeout,
        }
    }

    pub fn fetch_range(&self, start_byte: usize) -> Result<(u16, Vec<u8>), Box<dyn Error>> {
        let target = format!("{}:{}", self.host, self.port);
        let socket_addr: SocketAddr = target
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| format!("Failed to resolve address: {}", target))?;
        let mut stream = TcpStream::connect_timeout(&socket_addr, self.connect_timeout)?;
        stream.set_read_timeout(Some(self.read_write_timeout))?;
        stream.set_write_timeout(Some(self.read_write_timeout))?;
        Self::fetch_range_via_stream(&mut stream, &target, start_byte)
    }

    fn fetch_range_via_stream<T: Read + Write>(
        stream: &mut T,
        target_host: &str,
        start_byte: usize,
    ) -> Result<(u16, Vec<u8>), Box<dyn Error>> {
        let request = format!(
            "GET / HTTP/1.1\r\n\
             Host: {}\r\n\
             Range: bytes={}-\r\n\
             Connection: close\r\n\
             User-Agent: RustStdNetClient/1.0\r\n\
             \r\n",
            target_host, start_byte
        );
        stream.write_all(request.as_bytes())?;
        stream.flush()?;
        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        if reader.read_line(&mut status_line)? == 0 {
            return Err("Connection closed before status line received".into());
        }
        let status_code = parse_status_line(&status_line)?;
        let mut header_line = String::new();
        loop {
            header_line.clear();
            let bytes_read = reader.read_line(&mut header_line)?;
            if bytes_read == 0 {
                return Err("Connection closed during header reading".into());
            }
            if header_line == "\r\n" {
                break;
            }
        }
        let mut body_bytes = Vec::new();
        let mut chunk_buffer = [0; 8 * 1024];
        loop {
            match reader.read(&mut chunk_buffer) {
                Ok(0) => break,
                Ok(n) => body_bytes.extend_from_slice(&chunk_buffer[..n]),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(ref e)
                    if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
                {
                    eprintln!("\nWarning: Read timeout/wouldblock occurred during body read. Treating as partial read ({} bytes received this attempt).", body_bytes.len());
                    break;
                }
                Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => {
                    eprintln!("\nWarning: Unexpected EOF during body read. Treating as partial read ({} bytes received this attempt).", body_bytes.len());
                    break;
                }
                Err(e) => return Err(Box::new(e)),
            }
        }
        Ok((status_code, body_bytes))
    }
}

fn parse_status_line(line: &str) -> Result<u16, Box<dyn Error>> {
    let trimmed_line = line.trim();
    if trimmed_line.is_empty() {
        return Err("Status line is empty after trimming".into());
    }
    let parts: Vec<&str> = trimmed_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(format!("Malformed status line (too few parts): '{}'", trimmed_line).into());
    }
    if !parts[0].starts_with("HTTP/") {
        return Err(format!(
            "Malformed status line (invalid or missing HTTP version part '{}'): '{}'",
            parts[0], trimmed_line
        )
        .into());
    }
    parts[1].parse::<u16>().map_err(|e| {
        format!(
            "Invalid status code '{}' in line '{}': {}",
            parts[1], trimmed_line, e
        )
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io::{Error as IoError, Result as IoResult};

    struct MockTcpStream {
        read_queue: VecDeque<IoResult<Vec<u8>>>,
        write_buffer: Vec<u8>,
    }

    impl MockTcpStream {
        fn new(responses: Vec<IoResult<Vec<u8>>>) -> Self {
            MockTcpStream {
                read_queue: responses.into(),
                write_buffer: Vec::new(),
            }
        }
    }

    impl Read for MockTcpStream {
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            match self.read_queue.pop_front() {
                Some(Ok(mut data)) => {
                    if data.is_empty() {
                        self.read_queue.push_front(Ok(Vec::new()));
                        Ok(0)
                    } else {
                        let bytes_to_copy = std::cmp::min(buf.len(), data.len());
                        buf[..bytes_to_copy].copy_from_slice(&data[..bytes_to_copy]);
                        if data.len() > bytes_to_copy {
                            self.read_queue
                                .push_front(Ok(data.split_off(bytes_to_copy)));
                        }
                        Ok(bytes_to_copy)
                    }
                }
                Some(Err(e)) => Err(e),
                None => Ok(0),
            }
        }
    }

    impl Write for MockTcpStream {
        fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
            self.write_buffer.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> IoResult<()> {
            Ok(())
        }
    }

    #[test]
    fn test_fetch_success_206_partial_content() {
        let response_body = b"some partial data".to_vec();
        let response_headers = format!(
            "HTTP/1.1 206 Partial Content\r\n\
             Content-Length: {}\r\n\
             Content-Range: bytes 100-116/1000\r\n\
             \r\n",
            response_body.len()
        );
        let mut mock_stream = MockTcpStream::new(vec![
            Ok(response_headers.into_bytes()),
            Ok(response_body.clone()),
        ]);
        let start_byte = 100;
        let target_host = "mock.server:8080";
        let result = HttpClient::fetch_range_via_stream(&mut mock_stream, target_host, start_byte);
        assert!(result.is_ok());
        let (status, body) = result.unwrap();
        assert_eq!(status, 206);
        assert_eq!(body, response_body);
        let request_str =
            String::from_utf8(mock_stream.write_buffer).expect("Request not valid UTF-8");
        assert!(request_str.starts_with("GET / HTTP/1.1\r\n"));
        assert!(request_str.contains(&format!("\r\nHost: {}\r\n", target_host)));
        assert!(request_str.contains(&format!("\r\nRange: bytes={}-\r\n", start_byte)));
        assert!(request_str.contains("\r\nConnection: close\r\n"));
        assert!(request_str.ends_with("\r\n\r\n"));
    }

    #[test]
    fn test_fetch_success_200_ok() {
        let response_body = b"complete file data".to_vec();
        let response_headers = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Length: {}\r\n\
             \r\n",
            response_body.len()
        );
        let mut mock_stream = MockTcpStream::new(vec![
            Ok(response_headers.into_bytes()),
            Ok(response_body.clone()),
        ]);
        let start_byte = 0;
        let target_host = "mock.server:8080";
        let result = HttpClient::fetch_range_via_stream(&mut mock_stream, target_host, start_byte);
        assert!(result.is_ok());
        let (status, body) = result.unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, response_body);
        let request_str = String::from_utf8(mock_stream.write_buffer).unwrap();
        assert!(request_str.contains(&format!("\r\nRange: bytes={}-\r\n", start_byte)));
    }

    #[test]
    fn test_fetch_error_404_not_found() {
        let response_body = b"Not Found Error Page".to_vec();
        let response_headers = format!(
            "HTTP/1.1 404 Not Found\r\n\
             Content-Type: text/plain\r\n\
             Content-Length: {}\r\n\
             \r\n",
            response_body.len()
        );
        let mut mock_stream = MockTcpStream::new(vec![
            Ok(response_headers.into_bytes()),
            Ok(response_body.clone()),
        ]);
        let start_byte = 0;
        let target_host = "mock.server:8080";
        let result = HttpClient::fetch_range_via_stream(&mut mock_stream, target_host, start_byte);
        assert!(result.is_ok());
        let (status, body) = result.unwrap();
        assert_eq!(status, 404);
        assert_eq!(body, response_body);
    }

    #[test]
    fn test_fetch_simulated_timeout_during_body_read() {
        let response_part1 = b"first chunk".to_vec();
        let response_headers = format!(
            "HTTP/1.1 206 Partial Content\r\n\
             Content-Length: 1000\r\n\
             \r\n"
        );
        let mut mock_stream = MockTcpStream::new(vec![
            Ok(response_headers.into_bytes()),
            Ok(response_part1.clone()),
            Err(IoError::new(ErrorKind::TimedOut, "Simulated read timeout")),
        ]);
        let start_byte = 0;
        let target_host = "mock.server:8080";
        let result = HttpClient::fetch_range_via_stream(&mut mock_stream, target_host, start_byte);
        assert!(result.is_ok());
        let (status, body) = result.unwrap();
        assert_eq!(status, 206);
        assert_eq!(body, response_part1);
    }

    #[test]
    fn test_fetch_simulated_unexpected_eof_during_body_read() {
        let response_part1 = b"partial data before EOF".to_vec();
        let response_headers = format!(
            "HTTP/1.1 206 Partial Content\r\n\
             Content-Length: 1000\r\n\
             \r\n"
        );
        let mut mock_stream = MockTcpStream::new(vec![
            Ok(response_headers.into_bytes()),
            Ok(response_part1.clone()),
            Err(IoError::new(
                ErrorKind::UnexpectedEof,
                "Simulated unexpected EOF",
            )),
        ]);
        let start_byte = 0;
        let target_host = "mock.server:8080";
        let result = HttpClient::fetch_range_via_stream(&mut mock_stream, target_host, start_byte);
        assert!(result.is_ok());
        let (status, body) = result.unwrap();
        assert_eq!(status, 206);
        assert_eq!(body, response_part1);
    }

    #[test]
    fn test_fetch_premature_eof_before_status_line() {
        let mut mock_stream = MockTcpStream::new(vec![Ok(Vec::new())]);
        let start_byte = 0;
        let target_host = "mock.server:8080";
        let result = HttpClient::fetch_range_via_stream(&mut mock_stream, target_host, start_byte);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Connection closed before status line received"));
    }

    #[test]
    fn test_fetch_premature_eof_during_headers() {
        let response_partial = "HTTP/1.1 206 OK\r\nContent-Type: text/p";
        let mut mock_stream = MockTcpStream::new(vec![Ok(response_partial.as_bytes().to_vec())]);
        let start_byte = 0;
        let target_host = "mock.server:8080";
        let result = HttpClient::fetch_range_via_stream(&mut mock_stream, target_host, start_byte);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Connection closed during header reading"));
    }

    #[test]
    fn test_parse_status_line_valid_codes() {
        assert_eq!(parse_status_line("HTTP/1.1 200 OK\r\n").unwrap(), 200);
        assert_eq!(
            parse_status_line("HTTP/1.0 206 Partial Content").unwrap(),
            206
        );
        assert_eq!(parse_status_line("HTTP/2 404 Not Found").unwrap(), 404);
        assert_eq!(
            parse_status_line("HTTP/1.1 500 Internal Server Error").unwrap(),
            500
        );
        assert_eq!(
            parse_status_line(" HTTP/1.1 302 Found Redirect \r\n").unwrap(),
            302
        );
    }

    #[test]
    fn test_parse_status_line_invalid_format() {
        assert!(parse_status_line("HTTP/1.1 OK").is_err());
        assert!(parse_status_line("HTTP/1.1 20X OK").is_err());
        assert!(parse_status_line(" 200 OK").is_err());
        assert!(parse_status_line("HTTP/1.1").is_err());
        assert!(parse_status_line("").is_err());
        assert!(parse_status_line("\r\n").is_err());
    }
}
