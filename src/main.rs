use std::net::{Shutdown, TcpListener, TcpStream};
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::thread;
use std::str;
use std::collections::HashMap;
use std::time::SystemTime;
use maplit::hashmap;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
//use serde_json::json;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use sha256::digest;

struct LoginData {
    client_token: String,
    server_token: String,
    client_hash: String,
    username: String
}

#[derive(Serialize, Deserialize)]
enum MessageType {
    SendToken,
    RequestToken,
    Username,
    LoginRequest,
    Result(OperationResult)
}

#[derive(Serialize, Deserialize)]
enum OperationResult {
    Success,
    Fail
}

#[derive(Serialize, Deserialize)]
struct Message {
    header: HashMap<String, String>,
    msg_type: MessageType,
    body: Option<HashMap<String, String>>
}

fn read_message(stream: &mut TcpStream) -> Result<Message, serde_json::Error> {
    // Read 1 byte representing message length
    let mut len_buf = vec![0u8; 1];
    stream.read_exact(&mut len_buf).expect("Unable to read message length");

    // Convert the received byte to a usize
    let len = len_buf[0] as char;
    let len: usize = len.to_digit(10).unwrap() as usize;

    // Read the specified amount of bytes from the stream
    let mut msg_buf = vec![0u8; len];
    stream.read_exact(&mut msg_buf).expect("Unable to read message");
    
    // Convert the received bytes into a string
    let msg = match str::from_utf8(&msg_buf[..]) {
        Ok(msg) => msg,
        Err(_) => panic! ("Unable to convert message to string")
    };

    // Flush the stream
    stream.flush().expect("Unable to flush stream");

    // Parse the Json string into a Message object
    serde_json::from_str(&msg)
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
        let json = match serde_json::to_string(self) {
            Ok(s) => s,
            Err(e) => 
                return Err(Error::new(ErrorKind::Other, e))
        };

        // Convert the serialized json into bytes
        let serialized = json.as_bytes();

        // Write the length of the message first so the recipient knows how many bytes to listen for
        let len = serialized.len().to_string();
        let len = len.as_bytes();
        stream.write(len).expect("Unable to write message length");

        stream.write_all(serialized)
    }
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
    return format!("{}", utc_time.format("%d/%m/%Y %T"));
}

fn get_server_secret(username: String) -> &str {

}

/*
    Client connects to server and send a random token (T1)
    Server sends a random token (T2)
    Client send M = sha( T1 + T2 + < password: client's copy > ) to the server
    The server checks if sha( T1 + T2 + < password: server's copy > ) matches M (the hash just received).
*/

fn process_login_request(data: LoginData) -> MessageType {
    if data.username.is_empty() {
        return MessageType::Result(OperationResult::Fail);
    }
    if data.client_token.is_empty() {
        return MessageType::Result(OperationResult::Fail);
    }
    if data.client_hash.is_empty() {
        return MessageType::Result(OperationResult::Fail);
    }

    let server_hash = format!("{}{}{}", 
        data.client_token, 
        data.server_token, 
        get_server_secret(data.username)
    );
    let server_hash: String = digest(server_hash);
    if server_hash == data.client_hash {
        return MessageType::Result(OperationResult::Success);
    }
    else {
        return MessageType::Result(OperationResult::Fail);
    }
}

#[allow(unreachable_code)]
fn handle_client(mut stream: TcpStream) {
    // Let the client know they're connected
    let msg_connected = Message {
        header: hashmap!{
            String::from("timestamp") => get_timestamp()
        },
        msg_type: MessageType::Result(OperationResult::Success),
        body: None
    };

    msg_connected.send(&mut stream).expect("Unable to send message");

    let mut login_data = LoginData {
        username: String::new(),
        client_token: String::new(),
        server_token: gen_token(),
        client_hash: String::new()
    };

    loop {
        let msg: Message = read_message(&mut stream)
            .expect("Unable to read message");

        match msg.msg_type {
            MessageType::SendToken => 
                login_data.client_token = msg.get_body_value("token"),
            MessageType::RequestToken => {
                Message {
                    header: hashmap! {
                        String::from("timestamp") => get_timestamp()
                    },
                    msg_type: MessageType::SendToken,
                    body: Some(hashmap! {
                        String::from("token") => login_data.server_token.clone()
                    })
                }.send(&mut stream).expect("Unable to send token")
            },
            MessageType::Username => 
                login_data.username = msg.get_body_value("username"),
            MessageType::LoginRequest => {
                login_data.client_hash = msg.get_body_value("client_hash");

                Message {
                    header: hashmap! {
                        String::from("timestamp") => get_timestamp()
                    },
                    msg_type: process_login_request(login_data),
                    body: None
                }.send(&mut stream).expect("Unable to send login result")
            },
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