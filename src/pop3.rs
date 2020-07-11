use std::net::{IpAddr, Ipv4Addr, TcpStream};


struct POP3Emclient {
    host_name: String,
    host_ip: Ipv4Addr,
}

impl POP3Emclient{
    fn connect(&mut self){
        if let IpAddr::V4(ip) = &self.host_ip{
            let copy = ip.clone();
            //println!(copy);
            let client = TcpStream::connect((copy, 110));
        }

    }
}