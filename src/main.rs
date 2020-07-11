mod tcp;
mod smtp;
mod util;
mod pop3;

extern crate regex;
extern crate chrono;

use std::net::{TcpStream, Shutdown};
use std::io::{Write, Read, BufReader, BufRead};

use chrono::prelude::*;
//pop3:MVHEFVZAMGXLJXBR

fn main() {

    pop3::run();
}
