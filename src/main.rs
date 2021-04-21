use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::str;
use std::time::SystemTime;
use maplit::hashmap;
use chrono::DateTime;
use chrono::offset::Utc;
use serde::{Deserialize, Serialize};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use sha256::digest;
use mysql::*;
use mysql::prelude::*;
//use serde_json::json;

// Import the Messaging module
use networking;

#[derive(Serialize, Deserialize, Debug)]
struct LoginData {
    client_token: String,
    server_token: String,
    client_hash: String,
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
    return format!("{}", utc_time.format("%d/%m/%Y %T"));
}

fn get_server_secret(username: &str) -> String {
    let db_url = "mysql://rust:12345@server:3306/rust_login_service";

    let pool = Pool::new(db_url).unwrap();

    let mut conn = pool.get_conn().unwrap();

    let query: String = format!("
        SELECT Password 
        FROM users
        WHERE Username = \"{}\";", username
    );

    return match conn.query_first(query).unwrap() {
        Some(pw) => pw,
        None => panic!("User not found")
    };
}

/*
    Client connects to server and send a random token (T1)
    Server sends a random token (T2)
    Client send M = sha( T1 + T2 + < password: client's copy > ) to the server
    The server checks if sha( T1 + T2 + < password: server's copy > ) matches M (the hash just received).
*/

fn process_login_request(data: &LoginData) -> networking::MessageType {
    if data.username.is_empty() ||
        data.client_token.is_empty() ||
        data.client_hash.is_empty() {
        return networking::MessageType::Result(networking::OperationResult::Fail);
    }

    println!("Processing login request for: {:?}", data);

    let server_hash = format!("{}{}{}", 
        data.client_token, 
        data.server_token, 
        get_server_secret(&data.username)
    );
    let server_hash: String = digest(server_hash);
    if server_hash == data.client_hash {
        return networking::MessageType::Result(networking::OperationResult::Success);
    }
    else {
        return networking::MessageType::Result(networking::OperationResult::Fail);
    }
}

#[allow(unreachable_code)]
fn handle_client(mut stream: TcpStream) {
    // Let the client know they're connected
    let msg_connected = networking::Message {
        header: hashmap!{
            String::from("timestamp") => get_timestamp()
        },
        msg_type: networking::MessageType::Result(networking::OperationResult::Success),
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
        let msg: networking::Message = 
            networking::Message::read_into_message(&mut stream)
            .expect("Unable to read message");

        println!("Message type: {:?}", msg.msg_type);

        match msg.msg_type {
            networking::MessageType::SendToken => 
                login_data.client_token = msg.get_body_value("token"),
            networking::MessageType::RequestToken => {
                networking::Message {
                    header: hashmap! {
                        String::from("timestamp") => get_timestamp()
                    },
                    msg_type: networking::MessageType::SendToken,
                    body: Some(hashmap! {
                        String::from("token") => login_data.server_token.clone()
                    })
                }.send(&mut stream).expect("Unable to send token")
            },
            networking::MessageType::Username => 
                login_data.username = msg.get_body_value("username"),
                networking::MessageType::LoginRequest => {
                login_data.client_hash = msg.get_body_value("client_hash");

                networking::Message {
                    header: hashmap! {
                        String::from("timestamp") => get_timestamp()
                    },
                    msg_type: process_login_request(&login_data),
                    body: None
                }.send(&mut stream).expect("Unable to send login result")
            },
            networking::MessageType::Result(_r) => 
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