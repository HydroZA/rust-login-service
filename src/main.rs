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

fn remove_leading_zeroes(s: &str) -> &str {
    let mut out = "";
    for (i, c) in s.chars().enumerate() {
        if c == '0' {
            out = &s[i..];
        }
        else {
            break;
        }
    }
    out
}

fn read_message(stream: &mut TcpStream) -> std::result::Result<Message, serde_json::Error> {
    // Read 1 byte representing message length
    let mut len_buf = vec![0u8; 4];
    stream.read_exact(&mut len_buf).expect("Unable to read message length");

    // Convert the received byte to a usize
    let len: &str = str::from_utf8(&len_buf[..])
        .unwrap()
        .trim();
    
    let len: usize = remove_leading_zeroes(&len)
        .parse()
        .unwrap();

 //   println!("{}", len);

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
        // convert self into json
        // we exit the function if this fails
        let json = match serde_json::to_string(self) {
            Ok(s) => s,
            Err(e) => 
                return Err(Error::new(ErrorKind::Other, e))
        };

        // Convert the json into bytes
        let serialized = json.as_bytes();

        // Write the length of the message first so the recipient knows how many bytes to listen for
        let len = serialized.len().to_string();

        // pad len to always be 4 bytes long
        let len: String = format!("{:0>4}", len);

     //   println!("{}", len);
        
        stream.write(len.as_bytes()).expect("Unable to write message length");

        stream.write_all(serialized)
    }
}

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
    match read_message(&mut con) {
        Ok(m) => match m.msg_type {
            MessageType::Result(OperationResult::Success) => {},
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
    Message {
        msg_type: MessageType::SendToken,
        header: hashmap! {
            String::from("timestamp") => get_timestamp()
        },
        body: Some(hashmap! {
            String::from("token") => login_data.client_token.clone()
        })
    }.send(&mut con);


    // Request the servers token
    Message {
        msg_type: MessageType::RequestToken,
        header: hashmap! {
            String::from("timestamp") => get_timestamp()
        },
        body: None
    }.send(&mut con);

    login_data.server_token = match read_message(&mut con) {
        Ok(t) => t.get_body_value("token"),
        Err(_) => panic!("Unable to read server token")
    };

    // tell the server the username we are attempting to login with
    Message {
        msg_type: MessageType::Username,
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
    Message {
        msg_type: MessageType::LoginRequest,
        header: hashmap! {
            String::from("timestamp") => get_timestamp()
        },
        body: Some(hashmap! {
            String::from("client_hash") => auth_hash
        })
    }.send(&mut con);

    match read_message(&mut con).unwrap().msg_type {
        MessageType::Result(OperationResult::Success) => println!("Login Successful!"),
        _ => println!("Login failed!")
    };
}
