use std::net::{TcpListener, TcpStream};
use std::thread;

enum MessageType {
    Token,
    LoginDetails
}

struct Header {
    timestamp: String,
    msg_type: MessageType
}

struct Body {
    
}

struct Message {
    header: Header,
    body: Body
}

fn handle_client(stream: TcpStream) {
    
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:6969").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| handle_client(stream));
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }
}