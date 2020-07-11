use std::io::{BufReader, Write, BufRead};
use std::net::TcpStream;
use std::ops::Add;
use crate::util::MailError;
use std::net::Shutdown::Both;


pub struct Client {
    host_name: String,
    port: u16,
    connection: Option<BufReader<TcpStream>>,
    end_pointer: String
}

impl Client{

    pub fn new(host_name: String, port: u16) -> Client{
        return Client{
            host_name,
            port,
            connection: None,
            end_pointer: String::from("")
        }
    }

    pub fn set_port(&mut self, port: u16){
        self.port = port;
    }

    pub fn set_end_pointer(&mut self, end_pointer: String){
        self.end_pointer = end_pointer;
    }

    pub fn connect(&mut self) -> Result<(), MailError>{
        let copy = self.host_name.clone();
        return if let Ok(client) = TcpStream::connect((copy.as_str(), self.port)) {
            self.connection.replace(BufReader::new(client));
            Ok(())
        } else {
            self.connection.take();
            Err(MailError::TCPConnectFailError)
        }
    }

    pub fn send(&mut self, content: String) -> Result<(), MailError>{
        return if let Some(connection) = &mut self.connection {
            let content = content.add(&self.end_pointer);
            return if let Ok(k) = connection.get_mut().write(content.as_bytes()) {
                Ok(())
            } else {
                Err(MailError::TCPWriteError)
            }
        } else {
            Err(MailError::TCPNoConnectionError)
        }
    }

    pub fn shutdown(&mut self){
        if let Some(connection) = self.connection.take(){
            connection.into_inner().shutdown(Both);
        }
    }

    pub fn receive(&mut self) -> Result<String, MailError>{
        if let Some(mut connection) = self.connection.take(){
            let mut buf: Vec<u8> = Vec::new();
            connection.read_until(b'\n', &mut buf);
            self.connection.replace(connection);
            return if let Ok(s) = String::from_utf8(buf) {
                Ok(s)
            } else {
                Err(MailError::TCPFromUTF8Error)
            }
        }
        return Err(MailError::TCPNoConnectionError);
    }

    pub fn error_handler(&mut self, error: MailError) -> Result<(), MailError>{
        match error{
            MailError::TCPNoConnectionError => {
                if let Err(error) = self.connect(){
                    return Err(error);
                }
            },
            MailError::TCPConnectFailError => {
                if let Err(error) = self.connect(){
                    return Err(error);
                }
            },
            _ => { return Err(error); },
        }
        Ok(())
    }

}