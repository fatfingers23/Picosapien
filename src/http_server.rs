use core::str;

use defmt::*;
use embassy_net::{tcp::TcpSocket, Stack};
use embassy_time::Duration;
use embedded_io_async::Write;
use httparse::Header;

pub struct HttpServer {
    port: u16,
    stack: Stack<'static>,
}

impl HttpServer {
    pub fn new(port: u16, stack: Stack<'static>) -> Self {
        Self { port, stack }
    }

    pub async fn serve<H>(&mut self, mut handler: H)
    where
        H: WebRequestHandler,
    {
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

                let mut headers = [httparse::EMPTY_HEADER; 10];

                let request = self.request_parser(&mut buf[..n], &mut headers);
                match request {
                    Some(request) => {
                        let response = handler.handle_request(request).await;
                        match socket.write_all(response.as_bytes()).await {
                            Ok(()) => {}
                            Err(e) => {
                                warn!("write error: {:?}", e);
                                break;
                            }
                        };
                    }
                    None => {
                        warn!("Was not a proper web request");
                    }
                }

                //Have to close the socket so the web browser knows its done
                socket.close();
            }
        }
    }
    pub fn request_parser<'headers, 'buf>(
        &mut self,
        request_buffer: &'buf [u8],
        headers: &'headers mut [Header<'buf>],
    ) -> Option<WebRequest<'headers, 'buf>> {
        let mut request: httparse::Request<'headers, 'buf> = httparse::Request::new(headers);
        let res = request.parse(request_buffer).unwrap();
        if res.is_partial() {
            info!("Was not a proper web request");
            return None;
        }

        // Split the request buffer into headers and body
        let mut headers_end = 0;
        for window in request_buffer.windows(4) {
            if window == b"\r\n\r\n" {
                headers_end = window.as_ptr() as usize - request_buffer.as_ptr() as usize + 4;
                break;
            }
        }

        let body = &request_buffer[headers_end..];

        Some(WebRequest {
            method: Method::new(request.method.unwrap()),
            path: request.path,
            body: match core::str::from_utf8(body) {
                Ok(body) => body,
                Err(_) => "",
            },
            headers: request.headers,
        })
    }
}

pub struct WebRequest<'headers, 'buf> {
    pub method: Option<Method>,
    pub path: Option<&'buf str>,
    pub body: &'buf str,
    pub headers: &'headers mut [Header<'buf>],
}

pub trait WebRequestHandler {
    async fn handle_request(&mut self, request: WebRequest) -> &str;
}

pub enum Method {
    Delete,
    Get,
    Head,
    Post,
    Put,
    Connect,
    Options,
    Trace,
    Copy,
    Lock,
    MkCol,
    Move,
    Propfind,
    Proppatch,
    Search,
    Unlock,
    Bind,
    Rebind,
    Unbind,
    Acl,
    Report,
    MkActivity,
    Checkout,
    Merge,
    MSearch,
    Notify,
    Subscribe,
    Unsubscribe,
    Patch,
    Purge,
    MkCalendar,
    Link,
    Unlink,
}

impl Method {
    pub fn new(method: &str) -> Option<Self> {
        if method.eq_ignore_ascii_case("Delete") {
            Some(Self::Delete)
        } else if method.eq_ignore_ascii_case("Get") {
            Some(Self::Get)
        } else if method.eq_ignore_ascii_case("Head") {
            Some(Self::Head)
        } else if method.eq_ignore_ascii_case("Post") {
            Some(Self::Post)
        } else if method.eq_ignore_ascii_case("Put") {
            Some(Self::Put)
        } else if method.eq_ignore_ascii_case("Connect") {
            Some(Self::Connect)
        } else if method.eq_ignore_ascii_case("Options") {
            Some(Self::Options)
        } else if method.eq_ignore_ascii_case("Trace") {
            Some(Self::Trace)
        } else if method.eq_ignore_ascii_case("Copy") {
            Some(Self::Copy)
        } else if method.eq_ignore_ascii_case("Lock") {
            Some(Self::Lock)
        } else if method.eq_ignore_ascii_case("MkCol") {
            Some(Self::MkCol)
        } else if method.eq_ignore_ascii_case("Move") {
            Some(Self::Move)
        } else if method.eq_ignore_ascii_case("Propfind") {
            Some(Self::Propfind)
        } else if method.eq_ignore_ascii_case("Proppatch") {
            Some(Self::Proppatch)
        } else if method.eq_ignore_ascii_case("Search") {
            Some(Self::Search)
        } else if method.eq_ignore_ascii_case("Unlock") {
            Some(Self::Unlock)
        } else if method.eq_ignore_ascii_case("Bind") {
            Some(Self::Bind)
        } else if method.eq_ignore_ascii_case("Rebind") {
            Some(Self::Rebind)
        } else if method.eq_ignore_ascii_case("Unbind") {
            Some(Self::Unbind)
        } else if method.eq_ignore_ascii_case("Acl") {
            Some(Self::Acl)
        } else if method.eq_ignore_ascii_case("Report") {
            Some(Self::Report)
        } else if method.eq_ignore_ascii_case("MkActivity") {
            Some(Self::MkActivity)
        } else if method.eq_ignore_ascii_case("Checkout") {
            Some(Self::Checkout)
        } else if method.eq_ignore_ascii_case("Merge") {
            Some(Self::Merge)
        } else if method.eq_ignore_ascii_case("MSearch") {
            Some(Self::MSearch)
        } else if method.eq_ignore_ascii_case("Notify") {
            Some(Self::Notify)
        } else if method.eq_ignore_ascii_case("Subscribe") {
            Some(Self::Subscribe)
        } else if method.eq_ignore_ascii_case("Unsubscribe") {
            Some(Self::Unsubscribe)
        } else if method.eq_ignore_ascii_case("Patch") {
            Some(Self::Patch)
        } else if method.eq_ignore_ascii_case("Purge") {
            Some(Self::Purge)
        } else if method.eq_ignore_ascii_case("MkCalendar") {
            Some(Self::MkCalendar)
        } else if method.eq_ignore_ascii_case("Link") {
            Some(Self::Link)
        } else if method.eq_ignore_ascii_case("Unlink") {
            Some(Self::Unlink)
        } else {
            None
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Delete => "DELETE",
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Connect => "CONNECT",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
            Self::Copy => "COPY",
            Self::Lock => "LOCK",
            Self::MkCol => "MKCOL",
            Self::Move => "MOVE",
            Self::Propfind => "PROPFIND",
            Self::Proppatch => "PROPPATCH",
            Self::Search => "SEARCH",
            Self::Unlock => "UNLOCK",
            Self::Bind => "BIND",
            Self::Rebind => "REBIND",
            Self::Unbind => "UNBIND",
            Self::Acl => "ACL",
            Self::Report => "REPORT",
            Self::MkActivity => "MKACTIVITY",
            Self::Checkout => "CHECKOUT",
            Self::Merge => "MERGE",
            Self::MSearch => "MSEARCH",
            Self::Notify => "NOTIFY",
            Self::Subscribe => "SUBSCRIBE",
            Self::Unsubscribe => "UNSUBSCRIBE",
            Self::Patch => "PATCH",
            Self::Purge => "PURGE",
            Self::MkCalendar => "MKCALENDAR",
            Self::Link => "LINK",
            Self::Unlink => "UNLINK",
        }
    }
}
