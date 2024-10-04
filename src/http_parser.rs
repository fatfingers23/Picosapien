use defmt::*;

pub fn request_parser(request: &[u8]) -> WebRequest {
    let request_string = core::str::from_utf8(request).unwrap();
    info!("request_string {:?}", request_string);
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
