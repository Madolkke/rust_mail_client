use crate::{tcp, util};
use crate::smtp;
use crate::util::{MailError, print_error};
use regex::Regex;



#[derive(Debug)]
pub struct Response{
    code: usize,
    group: Vec<String>
}

impl Response{
    fn new(code: usize) -> Response{
        Response{
            code,
            group: Vec::new()
        }
    }

}

pub enum State{
    Init, TCPConnected, Authorized
}

pub struct SMTPMail{
    pub from: Option<String>,
    pub to: Option<String>,
    pub subject: Option<String>,
    pub content: Option<Vec<String>>
}

impl SMTPMail{
    pub fn new() -> SMTPMail{
        SMTPMail{
            from: None,
            to: None,
            subject: None,
            content: None
        }
    }

    pub fn check_integrity(&self) -> usize{
        if let Some(from) = &self.from{
            if let Some(to) = &self.to{
                if let Some(subject) = &self.subject{
                    if let Some(content) = &self.content{
                        return 0;
                    }
                    return 4;
                }
                return 3;
            }
            return 2;
        }
        return 1;
    }

}
impl std::clone::Clone for smtp::SMTPMail{
    fn clone(&self) -> Self {
        SMTPMail{
            from: self.from.clone(),
            to: self.to.clone(),
            subject: self.subject.clone(),
            content: self.content.clone()
        }
    }
}


pub struct Client{
    client: tcp::Client,
    name: String,
    account: Option<String>,
    password: Option<String>,
    mail_group: Vec<SMTPMail>,
    state: State,
    debug: bool
}

impl Client{

    pub fn print_debug(&mut self, response: &Response){
        if self.debug{
            println!("{:?}", response);
        }
    }

    pub fn debug(&mut self){
        self.debug = !self.debug;
        if self.debug{
            println!("已切换至 debug 模式");
        }else{
            println!("已切换至 普通 模式");
        }
    }

    pub fn new(host_name: String, name: String) -> smtp::Client{
        let mut c = Client{
            client: tcp::Client::new(host_name, 25),
            name,
            account: None,
            password: None,
            mail_group: Vec::new(),
            state: State::Init,
            debug: false
        };
        c.client.set_end_pointer(String::from("\r\n"));
        return c;
    }

    fn _receive_to_end(&mut self) -> Result<Response, MailError>{
        let response_regex = Regex::new(r"^(\d+)(.)(.+)").unwrap();
        let mut response_group: Vec<String> = Vec::new();
        let mut counter: usize = 0;
        let code = loop{
            let response_line = self.client.receive()?;
            let _cap = response_regex.captures(&response_line);
            if let Some(cap) = _cap{
                response_group.push(cap[3].to_string());
                if &cap[2] == " "{
                    break cap[1].trim().parse::<usize>().unwrap()
                }
            }else{
                return Err(MailError::SMTPResponseParseError(response_line));
            }
            counter += 1;
            if counter > 1000{
                return Err(MailError::SMTPResponseParseError(String::from("Receive overtime.")));
            }
        };
        Ok(Response{
            code,
            group: response_group
        })
    }

    fn _connect(&mut self) -> Result<Response, MailError>{
        self.client.connect()?;
        self._receive_to_end()
    }

    fn _ehlo(&mut self, domain: String) -> Result<Response, MailError>{
        self.client.send(format!("ehlo {}", domain))?;
        self._receive_to_end()
    }

    fn _auth(&mut self, auth_para: Option<String>) -> Result<Response, MailError>{
        if let Some(para) = auth_para{
            self.client.send(format!("auth {}", para))?;
            self._receive_to_end()
        }else{
            self.client.send("auth".to_string())?;
            self._receive_to_end()
        }
    }

    fn _data(&mut self) -> Result<Response, MailError>{
        self.client.send(String::from("data"))?;
        self._receive_to_end()
    }

    fn _mail_from(&mut self, sender: String) -> Result<Response, MailError>{
        self.client.send(format!("mail from:<{}>", sender))?;
        self._receive_to_end()
    }

    fn _rcpt_to(&mut self, receiver: String) -> Result<Response, MailError>{
        self.client.send(format!("rcpt to:<{}>", receiver))?;
        self._receive_to_end()
    }

    fn _send_base64(&mut self, string: String) -> Result<Response, MailError>{
        self.client.send(base64::encode(string))?;
        self._receive_to_end()
    }

    fn _send_lines(&mut self, lines: Vec<String>) -> Result<Response, MailError>{
        for line in lines{
            self.client.send(line)?;
        }
        self._receive_to_end()
    }

    fn _quit(&mut self) -> Result<Response, MailError>{
        self.client.send(String::from("quit"))?;
        self._receive_to_end()
    }

    //-------------以下为业务函数--------------------

    pub fn connect(&mut self){
        if let State::Init = self.state{
            if let Ok(response) = self._connect(){
                self.print_debug(&response);
                self.state = State::TCPConnected;
                println!("客户端 <{}> 连接成功", self.name);
            }else{
                println!("客户端 <{}> 发起连接失败，请使用 -reset 进行重置", self.name);
            }
        }else{
            println!("客户端 <{}> 处于已连接状态，若要重置连接，请使用 -reset 命令", self.name);
        }
    }

    pub fn login(&mut self){
        match self.state{
            State::Init => {
                println!("客户端 <{}> 尚未发起连接，请先尝试连接", self.name); return;
            }
            State::TCPConnected => {

            }
            State::Authorized => {
                if let Some(account) = &self.account{
                    println!("客户端 <{}> 已登录为 <{}> ，若要重新登录请使用 -reset 进行重置", self.name, account); return;
                }else{
                    println!("未找到客户端 <{}> 的邮箱账号，请尝试使用 -reset 进行重置", self.name); return;
                }
            }
        }

        let account = self.account.clone().unwrap();
        let password = self.password.clone().unwrap();

        let ehlo_res = self._ehlo(self.name.clone());
        if let Ok(ehlo_response) = ehlo_res{
            self.print_debug(&ehlo_response);
            if ehlo_response.code != 250{
                println!("客户端 <{}> 请求登录失败，错误信息: {:?}", self.name, ehlo_response);
            }
        }else if let Err(error) = ehlo_res{
            println!("客户端 <{}> 请求登录失败，错误信息: {:?}", self.name, error); return;
        }

        //进行auth
        self.account = Some(account.clone());
        self.password = Some(password.clone());
        //auth login
        let _res = self._auth(Some(String::from("login")));
        if let Ok(response) = _res{
            self.print_debug(&response);
            if response.code != 334{
                println!("客户端 <{}> 请求身份验证失败，错误信息: {:?}", self.name, response); return;
            }
        }else if let Err(error) = _res{
            println!("客户端 <{}> 请求身份验证失败，错误信息: {:?}", self.name, error); return;
        }
        //auth account
        let account_res = self._send_base64(account);
        if let Ok(account_response) = account_res{
            self.print_debug(&account_response);
            if account_response.code != 334{
                println!("客户端 <{}> 发送用户名失败，错误信息: {:?}", self.name, account_response); return;
            }
        }else if let Err(error) = account_res{
            println!("客户端 <{}> 发送用户名失败，错误信息: {:?}", self.name, error); return;
        }
        //auth password
        let pass_res = self._send_base64(password);
        if let Ok(pass_response) = pass_res{
            self.print_debug(&pass_response);
            if pass_response.code != 235{
                println!("客户端 <{}> 发送授权码失败, 错误信息: {:?}", self.name, pass_response); return;
            }
        }else if let Err(error) = pass_res{
            println!("客户端 <{}> 发送授权码失败，错误信息: {:?}", self.name, error); return;
        }

        self.state = State::Authorized;
        println!("客户端 <{}> 登录成功", self.name);
    }

    pub fn save(&mut self, mail: SMTPMail){
        println!("客户端 <{}> 已保存邮件 [{}]", self.name, mail.subject.clone().unwrap());
        self.mail_group.push(mail);
    }

    pub fn send(&mut self, order_number: usize){
        match self.state{
            State::Init => {
                println!("客户端 <{}> 尚未发起连接，请先尝试连接", self.name); return;
            }
            State::TCPConnected => {
                println!("客户端 <{}> 尚未登录，请先尝试登录", self.name); return;
            }
            State::Authorized => {

            }
        }
        if order_number > self.mail_group.len(){
            println!("客户端 <{}> 未找到序号为 [{}] 的邮件，请重试", self.name, order_number); return;
        }

        let target = self.mail_group.remove(order_number - 1);
        if target.check_integrity() != 0{
            println!("客户端 <{}> 检查邮件完整性出错，错误码: {}", self.name, target.check_integrity());
            self.save(target);
            return;
        }

        //发送 mail from:
        let from_res = self._mail_from(target.from.clone().unwrap());
        if let Ok(response) = from_res{
            self.print_debug(&response);
            if response.code != 250{
                println!("客户端 <{}> 请求发送邮件发送方失败，错误信息: {:?}", self.name, response); return;
            }
        }else if let Err(error) = from_res{
            println!("客户端 <{}> 请求发送邮件发送方失败，错误信息: {:?}", self.name, error); return;
        }

        //发送 rcpt to:
        let from_res = self._rcpt_to(target.to.clone().unwrap());
        if let Ok(response) = from_res{
            self.print_debug(&response);
            if response.code != 250{
                println!("客户端 <{}> 请求发送邮件接收方失败，错误信息: {:?}", self.name, response); return;
            }
        }else if let Err(error) = from_res{
            println!("客户端 <{}> 请求发送邮件接收方失败，错误信息: {:?}", self.name, error); return;
        }


        //发送data
        let data_res = self._data();
        if let Ok(response) = data_res{
            self.print_debug(&response);
            if response.code != 354{
                self.save(target);
                println!("客户端 <{}> 请求发送邮件内容失败，错误信息: {:?}", self.name, response); return;
            }
        }else if let Err(error) = data_res{
            self.save(target);
            println!("客户端 <{}> 请求发送邮件内容失败，错误信息: {:?}", self.name, error); return;
        }


        let subject = target.subject.clone().unwrap();
        //开始连续发送邮件主要内容
        let send_lines_res = self._send_lines(serialize(target));
        if let Ok(response) = send_lines_res{
            self.print_debug(&response);
            if response.code != 250{
                println!("客户端 <{}> 发送邮件内容时出错，错误信息: {:?}", self.name, response); return;
            }
        }else if let Err(error) = send_lines_res{
            println!("客户端 <{}> 发送邮件内容时出错，错误信息: {:?}", self.name, error); return;
        }

        println!("客户端 <{}> 已成功发送邮件 [{}]", self.name, subject);

    }

    pub fn show_mail_group(&mut self){
        if self.mail_group.len() == 0{
            println!("客户端 <{}> 的已保存邮件列表中无内容", self.name); return;
        }
        println!("客户端 <{}> 的已保存邮件列表: ", self.name);
        let mut counter: usize = 0;
        for mail in &self.mail_group{
            counter += 1;
            println!("[{}]  {}", counter, mail.subject.clone().unwrap());
        }
        println!();
    }

    pub fn quit(&mut self){
        let quit_res = self._quit();
        if let Ok(response) = quit_res{
            self.print_debug(&response);
            if response.code != 221{
                println!("客户端 <{}> 退出失败，错误信息: {:?}", self.name, response); return;
            }
        }else if let Err(error) = quit_res{
            println!("客户端 <{}> 退出失败，错误信息: {:?}", self.name, error); return;
        }
        self.client.shutdown();
        self.mail_group.clear();
        self.state = State::Init;
    }

    pub fn reset(&mut self){
        self.client.shutdown();
        self.debug = false;
        self.state = State::Init;
    }

}

fn parse_response_group(response_group: Vec<String>) -> Result<Response, MailError>{
    if response_group.len() == 0 { return Err(MailError::SMTPResponseParseError(String::from("No Response Group Found."))); }
    let response_regex = Regex::new(r"(\d+).(.+)").unwrap();
    let code = response_regex.captures(&response_group[0]).unwrap()[1].trim().parse::<usize>().unwrap();
    let mut group: Vec<String> = Vec::new();
    for line in response_group{
        let cap = response_regex.captures(&line).unwrap();
        group.push(cap[2].to_string());
    }
    return Ok(Response{
        code,
        group
    })
}

fn is_response_end(response: String) -> bool{
    let end_regex = Regex::new(r"^(\d+).(.+)").unwrap();
    return end_regex.is_match(&response);
}

fn serialize(mail: SMTPMail) -> Vec<String>{
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("from:<{}>", mail.from.clone().unwrap()));
    lines.push(format!("to:<{}>",mail.to.clone().unwrap()));
    lines.push(format!("subject:{}", mail.subject.clone().unwrap()));
    lines.push(String::from("\r\n"));
    lines.append(&mut mail.content.clone().unwrap());
    lines.push(String::from("\r\n"));
    lines.push(String::from("."));
    lines
}

pub fn run(name: String, account: String, password: String, host_name: String, mail_list: &mut Vec<SMTPMail>){
    let mut client = smtp::Client::new(host_name, name);
    client.mail_group.clone_from(mail_list);
    client.account.replace(account);
    client.password.replace(password);

    loop{
        let c = ||{
            return if let State::Authorized = client.state {
                "已登录".to_string()
            } else { "未登录".to_string() }
        };
        println!("正在运行: smtp客户端 <{}> ({})，使用 -help 获得帮助", client.name, c());
        let _input = util::input();
        let mut __input = _input.split_whitespace();
        if let Some(head) = __input.next(){
            match head{
                "-help" => {
                    println!("-login  自动发起连接并尝试登录");
                    println!("-list  显示邮件列表中内容");
                    println!("-send [order number(int):邮件列表序号]  发送指定序号的邮件");
                    println!("-reset  重置 smtp 客户端");
                    println!("-debug  切换模式(普通/debug)");
                    println!("-quit  退出 smtp 客户端");
                }
                "-login" => {
                    client.connect();
                    client.login();
                }
                "-list" => {
                    if client.mail_group.len() == 0{
                        println!("无已保存的邮件，可使用编辑器创建新的邮件"); continue;
                    }
                    println!("---已保存的邮件列表---");
                    let mut counter: usize = 0;
                    for mail in &client.mail_group{
                        counter += 1;
                        if let Some(sub) = &mail.subject{
                            println!("[{}] {}", counter, sub);
                        }else{
                            println!("[{}] * Mail No Subject *", counter);
                        }
                    }
                    println!();
                }
                "-send" => {
                    if let Some(param1) = __input.next(){
                        if let Ok(on) = param1.trim().parse::<usize>(){
                            client.send(on);
                        }else { print_error(); }
                    }else { print_error(); }
                }
                "-reset" => {
                    client.reset();
                }
                "-debug" => {
                    client.debug();
                }
                "-quit" => {
                    println!("将会退出 smtp 客户端，是否确认？(yes/no)");
                    let s = util::input();
                    if &s == "yes"{
                        break;
                    }else if &s == "no"{
                        continue;
                    }else { print_error(); }
                }
                _ => {
                    print_error();
                }
            }
        }else{ print_error();}
    }
    mail_list.clone_from(&client.mail_group);
}