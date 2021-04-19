use std::net::{Shutdown, TcpListener, TcpStream};
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::thread;
use std::collections::HashMap;
use std::time::SystemTime;
use maplit::hashmap;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
//use serde_json::json;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

#[derive(Serialize, Deserialize)]
enum MessageType {
    SendToken,
    RequestToken,
    Username,
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
            .as_ref()
            .unwrap()
            .get(key)
            .expect("Unable to find key")
            .to_string()
    }

    fn send(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        // write to the message to TCP stream
        // we exit the function if this fails
        let serialized: String = match serde_json::to_string(self) {
            Ok(s) => s,
            Err(e) => 
                return Err(Error::new(ErrorKind::Other, e))
        };

        stream.write_all(serialized.as_bytes())
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
    username: String
}

fn gen_token() -> String {
    return thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();
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

    msg_connected.send(&mut stream).expect("Unable to send message");

    let mut login_data = LoginData {
        username: String::new(),
        client_token: String::new(),
        server_token: gen_token()
    };

    loop {
        let mut msg: String = String::new();

        stream.read_to_string(&mut msg).expect("Unable to read message");

        // parse message to json
        let msg: Message = serde_json::from_str(&msg).expect("Unable to parse JSON");

        match msg.msg_type {
            MessageType::SendToken => 
                login_data.client_token = msg.get_body_value("token"),
            MessageType::RequestToken => {
                    Message {
                        header: hashmap! {
                        "timestamp" => get_timestamp()
                        },
                        msg_type: MessageType::SendToken,
                        body: hashmap! {
                            "token" => login_data.server_token
                        }
                    }.send(&mut stream)
                },
            MessageType::Username => 
                login_data.username = msg.get_body_value("username"),
            MessageType::LoginRequest => 
                panic!("Not Yet Implemented"),
            MessageType::Result(_r) => 
                panic!("Not Yet Implemented")
        };
    }

   
    let _ = stream.shutdown(Shutdown::Both);
}




fn main() {
    let listener = TcpListener::bind("0.0.0.0:6969").unwrap();

    println!("Listening!");

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