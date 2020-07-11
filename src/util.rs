use trust_dns_resolver::Resolver;
use trust_dns_resolver::config;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug)]
pub enum MailError{
    TCPFromUTF8Error, TCPWriteError, TCPNoConnectionError, TCPConnectFailError,
    NotHandledError,
    POP3ResponseParseError(String), POP3StatusParseError(String)
}

pub fn is_error_tcp_handled(error: MailError) -> bool {
    //检测是否为TCP客户端可处理的错误
    match error {
        MailError::TCPNoConnectionError => { true },
        MailError::TCPConnectFailError => { true },
        _ => { false }
    }
}


pub fn trim_ok_result(result: Result<String, String>) -> Result<String, String>{
    return match result{
        Ok(s) => {
            Ok(s.trim_end().to_string())
        }
        Err(s) => {
            Err(s)
        }
    }

}

pub fn dns_resolve(host_name: String) -> Result<Ipv4Addr, String>{
    let mut _host_name = host_name;
    _host_name.push('.');
    if let Ok(resolver) = Resolver::new(ResolverConfig::default(), ResolverOpts::default()) {
        if let Ok(response) = resolver.lookup_ip(&_host_name) {
            if let Some(address) = response.iter().next() {
                if address.is_ipv4(){
                    if let IpAddr::V4(ip) = address{
                        return Ok(ip);
                    }else{
                        return Err("No ipv4 address returned.".to_string());
                    }
                }else{
                    return Err("No ipv4 address returned.".to_string());
                }
            }else{
                return Err("No address returned.".to_string());
            }
        }else{
            return Err("Failed to get ip by host name.".to_string());
        }
    }else{
        return Err("Failed to create DNS Resolver, check the system resolver config.".to_string());
    }
}