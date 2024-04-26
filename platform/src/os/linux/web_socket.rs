use crate::{
    event::{HttpRequest},
    web_socket::WebSocketMessage,
    thread::SignalToUI,
};
use std::net::{SocketAddr, TcpStream};
use std::sync::{
    mpsc::{Sender},
    Arc,
    Mutex,
};
use std::io::{Write, BufReader, BufRead};
use std::collections::HashMap;
use makepad_http::websocket::{ServerWebSocketMessageHeader, ServerWebSocketMessageFormat, ServerWebSocket};

pub struct OsWebSocket{
    connection: WebSocketConnection,
    rx_sender: Sender<WebSocketMessage>,
}

pub fn extract_uri(url: &str) -> (&str, &str, &str) {
    match url.split_once("://") {
        Some((scheme, rest)) => {
            let (authority, rest) = match rest.split_once('/') {
                Some((authority, rest)) => (authority, rest),
                None => (rest, "")
            };
            (scheme, authority, rest)
        }
        None => {
            panic!("Invalid URI {}", url);
        }
    }
}

impl OsWebSocket{
    pub fn send_message(&mut self, message: WebSocketMessage)->Result<(),()>{
        let frame = match &message{
            WebSocketMessage::String(data)=>{
                let header = ServerWebSocketMessageHeader::from_len(data.len(), ServerWebSocketMessageFormat::Text, true);
                ServerWebSocket::build_message(header, &data.to_string().into_bytes())
            }
            WebSocketMessage::Binary(data)=>{
                let header = ServerWebSocketMessageHeader::from_len(data.len(), ServerWebSocketMessageFormat::Text, true);
                ServerWebSocket::build_message(header, &data)
            }
            _=>panic!()
        };

        println!("Sending frame: {:?}", frame.len());
        self.connection.send(&frame)
    }
                    
    pub fn open(_socket_id:u64, request: HttpRequest, rx_sender:Sender<WebSocketMessage>)->OsWebSocket{
        let (_scheme, authority, rest) = extract_uri(&request.url);

        let ws_client = WebSocketConnection::connect(authority, rest);

        let _recv_message = {
            let rx_sender = rx_sender.clone();
            let mut ws_client = ws_client.clone();
            std::thread::spawn(move || {
                loop {
                    if let Some((ty, data)) = ws_client.read() {
                        println!("Received type: {} data.len: {}", ty, data.len());
                        if ty == 0 {
                            let msg = WebSocketMessage::Binary(data.to_vec());
                            rx_sender.send(msg).unwrap();
                            SignalToUI::set_ui_signal();
                        } else {
                            let s = String::from_utf8(data.to_vec()).unwrap();
                            let msg = WebSocketMessage::String(s);
                            rx_sender.send(msg).unwrap();
                            SignalToUI::set_ui_signal();
                        }
                    }
                }
            })
        };

        OsWebSocket {
            rx_sender,
            connection: ws_client,
        }
    }
}

#[derive(Clone)]
struct WebSocketConnection {
    tcp_stream: Arc<Mutex<TcpStream>>,
    connected: bool,
}

impl WebSocketConnection {
    pub fn connect(host: &str, path: &str) -> Self {
        let socket_addr: SocketAddr = host.parse().unwrap();
        let mut tcp_stream = TcpStream::connect(socket_addr).unwrap();

        let connect_request = format!(
            "GET /{} HTTP/1.1\r\n\
            Host: {}\r\n\
            Connection: Upgrade\r\n\
            Upgrade: websocket\r\n\
            Sec-WebSocket-Version: 13\r\n\
            Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
            \r\n",
            path, host
        );

        tcp_stream.write_all(connect_request.as_bytes()).unwrap();

        Self {
            tcp_stream: Arc::new(Mutex::new(tcp_stream)),
            connected: false,
        }
    }
    
    pub fn read(&mut self) -> Option<(usize, &[u8])> {
        let stream = self.tcp_stream.lock().unwrap();
        let reader = BufReader::new(&*stream);

        let mut lines = reader.lines();

        if !self.connected {
            let connect_string = lines.next().unwrap().unwrap();

            let mut headers: HashMap<String, String> = HashMap::new();
            for line in lines {
                let line = line.unwrap();
                if line.is_empty() {
                    break;
                }

                let (key, value) = line.split_once(": ").unwrap();
                headers.insert(key.to_string(), value.to_string());
            }

            if connect_string != "HTTP/1.1 101 Switching Protocols" {
                // TODO: Panic instead?
                return None;
            }
            // TODO: Validate response headers

            self.connected = true;
            None
        } else {
            for line in lines {
                let line = line.unwrap();
                println!("line: {}", line);
            }
            None
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), ()> {
        let stream = self.tcp_stream.lock();
        println!("stream: {:?}", stream);

        let mut stream = stream.unwrap();

        println!("Got tcp_stream... sending");
        stream.write_all(data).unwrap();

        println!("Got tcp_stream... sending");
        Ok(())
    }
} 
