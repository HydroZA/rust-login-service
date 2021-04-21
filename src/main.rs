use std::io;
use std::io::{Error, ErrorKind};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::net::{TcpStream};
use std::io::{Read, Write};
use std::str;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use maplit::hashmap;
use chrono::DateTime;
use chrono::offset::Utc;
use sha256::digest;

// Import the networking module
mod networking;

struct LoginData {
    client_token: String,
    server_token: String,
    username: String,
    password: String
}

impl LoginData {
    fn get_hash(&self) -> String {
        return digest(format!("{}{}{}",
            self.client_token,
            self.server_token,
            self.password
        ));
    }
}

fn gen_token() -> String {
    return thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();
}

fn get_login_data() -> LoginData {
    println!("Enter username:");

    let mut username = String::new();
    
    io::stdin().read_line(&mut username)
        .expect("Failed to read line");

    // Remove any whitespace 
    let username = String::from(username.trim());

    println!("Enter password:");

    let mut password = String::new();
    
    io::stdin().read_line(&mut password)
        .expect("Failed to read line");

    // Remove any whitespace 
    let password = String::from(password.trim());

    LoginData {
        client_token: gen_token(),
        server_token: String::new(),
        username,
        password
    }
}

fn get_timestamp() -> String {
    // Get time for timestamp field
    let sys_time = SystemTime::now();
    let utc_time: DateTime<Utc> = sys_time.into();
    return format!("{}", utc_time.format("%d/%m/%Y %T"));
}

fn main() {
    let mut con = match TcpStream::connect("127.0.0.1:6969") {
        Ok(stream) => stream,
        Err(_) => panic!("Unable to connect to server")
    };

    // Verify Connection
    match networking::Message::read_into_message(&mut con) {
        Ok(m) => match m.msg_type {
            networking::MessageType::Result(networking::OperationResult::Success) => {},
            _ => panic!("Bad server response")
        },
        Err(_) => panic!("Bad server response")
    };

    println!("Connected!\n");

    // Get login details
    let mut login_data = get_login_data();

    // Generate a unique token
    login_data.client_token = gen_token();

    // Send our token to the server
    networking::Message {
        msg_type: networking::MessageType::SendToken,
        header: hashmap! {
            String::from("timestamp") => get_timestamp()
        },
        body: Some(hashmap! {
            String::from("token") => login_data.client_token.clone()
        })
    }.send(&mut con);


    // Request the servers token
    networking::Message {
        msg_type: networking::MessageType::RequestToken,
        header: hashmap! {
            String::from("timestamp") => get_timestamp()
        },
        body: None
    }.send(&mut con);

    login_data.server_token = match networking::Message::read_into_message(&mut con) {
        Ok(t) => t.get_body_value("token"),
        Err(_) => panic!("Unable to read server token")
    };

    // tell the server the username we are attempting to login with
    networking::Message {
        msg_type: networking::MessageType::Username,
        header: hashmap! {
            String::from("timestamp") => get_timestamp()
        },
        body: Some(hashmap! {
            String::from("username") => login_data.username.clone()
        })
    }.send(&mut con);

    // create our authentication hash
    let auth_hash = login_data.get_hash();

    // finally, create the login request
    networking::Message {
        msg_type: networking::MessageType::LoginRequest,
        header: hashmap! {
            String::from("timestamp") => get_timestamp()
        },
        body: Some(hashmap! {
            String::from("client_hash") => auth_hash
        })
    }.send(&mut con);

    match networking::Message::read_into_message(&mut con).unwrap().msg_type {
        networking::MessageType::Result(networking::OperationResult::Success) => println!("Login Successful!"),
        _ => println!("Login failed!")
    };
}
