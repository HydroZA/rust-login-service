use std::collections::HashMap;
use std::net::{TcpStream};
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use serde::{Deserialize, Serialize};
use std::str;

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    SendToken,
    RequestToken,
    Username,
    LoginRequest,
    Result(OperationResult)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OperationResult {
    Success,
    Fail
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub header: HashMap<String, String>,
    pub msg_type: MessageType,
    pub body: Option<HashMap<String, String>>
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

impl Message {
    pub fn read_into_message(stream: &mut TcpStream) -> std::result::Result<Message, serde_json::Error> {
        // Read 1 byte representing message length
        let mut len_buf = vec![0u8; 4];
        stream.read_exact(&mut len_buf).expect("Unable to read message length");
    
        // Convert the received byte to a usize
        let len: &str = str::from_utf8(&len_buf[..])
            .unwrap()
            .trim();
        
        let len: usize = remove_leading_zeroes(len)
            .parse()
            .unwrap();
    
        println!("Reading {} bytes", len);
        
            // Read the specified amount of bytes from the stream
        let mut msg_buf = vec![0u8; len];
        stream.read_exact(&mut msg_buf).expect("Unable to read message");
        
        println!("Got all bytes");
    
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

    // On the server side we will only ever have to retrieve the client token
    pub fn get_body_value(&self, key: &str) -> String {
        self.body
            .as_ref()
            .unwrap()
            .get(key)
            .expect("Unable to find key")
            .to_string()
    }

    pub fn send(&self, stream: &mut TcpStream) -> std::io::Result<()> {
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
        
        stream.write(len.as_bytes()).expect("Unable to write message length");

        stream.write_all(serialized)
    }
}
