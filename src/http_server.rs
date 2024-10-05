use core::str;

use defmt::*;
use embassy_net::{tcp::TcpSocket, Stack};
use embassy_time::Duration;
use embedded_io_async::Write;
use httparse::{Header, EMPTY_HEADER};

pub struct HttpServer {
    port: u16,
    stack: Stack<'static>,
}

impl HttpServer {
    pub fn new(port: u16, stack: Stack<'static>) -> Self {
        Self { port, stack }
    }

    pub async fn serve(&mut self) {
        let mut rx_buffer = [0; 4096];
        let mut tx_buffer = [0; 4096];
        let mut buf = [0; 4096];
        info!("Listening on port 80");
        loop {
            let mut socket = TcpSocket::new(self.stack, &mut rx_buffer, &mut tx_buffer);
            socket.set_timeout(Some(Duration::from_secs(10)));

            // control.gpio_set(0, false).await;

            if let Err(e) = socket.accept(self.port).await {
                warn!("accept error: {:?}", e);
                continue;
            }

            info!("Received connection from {:?}", socket.remote_endpoint());
            // control.gpio_set(0, true).await;

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => {
                        warn!("read EOF");
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        warn!("read error: {:?}", e);
                        break;
                    }
                };

                let request = request_parser(&buf[..n]);
                info!("request {:?}", request.path);
                // info!("Web request{}", from_utf8(&buf[..n]).unwrap());
                let html = "HTTP/1.0 200 OK\r\nContent-type: text/html\r\n\r\n<!DOCTYPE html>
                <html>
                    <body>
                       <h1>Pico W - Hello World!</h1>
                    </body>
                </html";

                match socket.write_all(html.as_bytes()).await {
                    Ok(()) => {}
                    Err(e) => {
                        warn!("write error: {:?}", e);
                        break;
                    }
                };
                //Have to close the socket so the web browser knows its done
                socket.close();
            }
        }
    }
}

pub fn request_parser(request_buffer: &[u8]) -> WebRequest {
    let request_string = core::str::from_utf8(request_buffer).unwrap();

    let mut headers = [httparse::EMPTY_HEADER; 10];
    let mut request: httparse::Request<'_, '_> = httparse::Request::new(&mut headers);
    let res = request.parse(request_buffer).unwrap();

    if res.is_complete() {
        // request.
        // info!("idk: {:?}", request);
        match request.path {
            Some(ref path) => {
                // check router for path.
                // /404 doesn't exist? we could stop parsing
            }
            None => {
                // must read more and parse again
            }
        }
    }

    info!("request_string {:?}", request_string);

    // Split the request buffer into headers and body
    let mut headers_end = 0;
    for window in request_buffer.windows(4) {
        if window == b"\r\n\r\n" {
            headers_end = window.as_ptr() as usize - request_buffer.as_ptr() as usize + 4;
            break;
        }
    }

    let body = &request_buffer[headers_end..];
    info!("Just body {:?}", core::str::from_utf8(body).unwrap());
    let mut lines = request_string.lines();
    let first_line = lines.next().unwrap();
    let mut first_line_words = first_line.split_whitespace();
    let method = match first_line_words.next().unwrap() {
        "POST" => HttpMethod::POST,
        "GET" => HttpMethod::Get,
        _ => HttpMethod::Unsupported,
    };
    let path = first_line_words.next().unwrap();
    let version = first_line_words.next().unwrap();

    //TODO need to read till content length. Parse number read that many bytes
    // let body = lines.next().unwrap();

    // info!("method {:?}", method);
    info!("path {:?}", path);
    info!("version {:?}", version);

    WebRequest {
        method,
        path,
        version,
        // body,
    }
}

pub enum HttpMethod {
    POST,
    Get,
    Unsupported,
}

pub struct WebRequest<'a> {
    pub method: HttpMethod,
    pub path: &'a str,
    pub version: &'a str,
    // body: &'a str,
}
