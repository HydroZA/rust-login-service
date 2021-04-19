use std::net::{Shutdown, TcpListener, TcpStream};
use std::io::prelude::*;
use std::thread;
use std::collections::HashMap;
use std::time::SystemTime;
use maplit::hashmap;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
enum MessageType {
    Token,
    LoginDetails,
    LoginRequest,
    Result(Result)
}

#[derive(Serialize, Deserialize)]
enum Result {
    Success,
    Fail
}

#[derive(Serialize, Deserialize)]
struct Message {
    header: HashMap<String, String>,
    msg_type: MessageType,
    body: Option<HashMap<String, String>>
}

impl Message {
    // On the server side we will only ever have to retrieve the client token
    fn get_body_value(&self, key: &str) -> String {
        self.body
            .unwrap()
            .get(key)
            .expect("Unable to find key")
            .to_string()
    }
}

/*
    Client connects to server and send a random token (T1)
    Server sends a random token (T2)
    Client send M = sha( T1 + T2 + < password: client's copy > ) to the server
    The server checks if sha( T1 + T2 + < password: server's copy > ) matches M (the hash just received).
*/

struct LoginData {
    client_token: String,
    server_token: String,
    username: String,
    client_secret: String,
    server_secret: String
}

fn get_timestamp() -> String {
    // Get time for timestamp field
    let sys_time = SystemTime::now();
    let utc_time: DateTime<Utc> = sys_time.into();
    let timestamp = format!("{}", utc_time.format("%d/%m/%Y %T"));

    timestamp
}

#[allow(unreachable_code)]
fn handle_client(mut stream: TcpStream) {
    // Let the client know they're connected
    let msg_connected = Message {
        header: hashmap!{
            String::from("timestamp") => get_timestamp()
        },
        msg_type: MessageType::Result(Result::Success),
        body: None
    };

    // write to the message to TCP stream
    // we exit the function if this fails
    let serialized: String = match serde_json::to_string(&msg_connected) {
        Ok(s) => s,
        Err(_) => return
    };
    stream.write_all(serialized.as_bytes()).expect("Unable to write to stream");

    let mut login_data = LoginData {
        username: String::new(),
        client_token: String::new(),
        server_token: String::new(),
        client_secret: String::new(),
        server_secret: String::new()
    };

    loop {
        let mut msg: String = String::new();

        stream.read_to_string(&mut msg).expect("Unable to read message");

        // parse message to json
        let msg: Message = serde_json::from_str(&msg).expect("Unable to parse JSON");



        match msg.msg_type {
            MessageType::Token => login_data.client_token = msg.get_body_value("client_token"),
            MessageType::LoginDetails => {
                login_data.username = msg.get_body_value("username");
                login_data.client_secret = msg.get_body_value("password")
            },
            MessageType::LoginRequest => 
        };
    }

   
    let _ = stream.shutdown(Shutdown::Both);
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