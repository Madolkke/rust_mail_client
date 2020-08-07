use crate::tcp;
use crate::pop3::State::TCPConnected;
use crate::util;
use crate::util::MailError;
use crate::util::print_error;
use regex::Regex;
use crate::util::MailError::POP3ResponseParseError;
use std::ops::Add;
use base64;
use encoding::{DecoderTrap, Encoding};
use encoding::all::GBK;

#[derive(Debug)]
enum Response{
    Ok(String),
    Err(String)
}

enum State{
    Init, TCPConnected, Authorized
}

pub struct Client{
    client: tcp::Client,
    state: State,
    name: String,
    account: Option<String>,
    password: Option<String>,
    dele_list: Vec<usize>,
    debug: bool
}

impl Client{

    pub fn new(host_name: String, name: String) -> Client{
        let mut c = Client{
            client: tcp::Client::new(host_name, 110),
            state: State::Init,
            name,
            account: None,
            password: None,
            dele_list: Vec::new(),
            debug: false
        };
        c.client.set_end_pointer(String::from("\r\n"));
        return c
    }

    pub fn print_debug(&mut self, response: &Response){
        if self.debug{
            println!("Response: {:?}", response);
        }
    }

    fn _connect(&mut self) -> Result<Response, MailError>{
        return match self.client.connect() {
            Ok(s) => {
                return match self.client.receive() {
                    Ok(s) => {
                        self.state = State::TCPConnected;
                        parse_response(s)
                    }
                    Err(s) => {
                        Err(s)
                    }
                }
            }
            Err(s) => {
                Err(s)
            }
        }
    }

    fn _noop(&mut self) -> Result<Response, MailError>{
        self.client.send(String::from("noop"))?;
        let _response = self.client.receive();
        if let Ok(response) = _response{
            return parse_response(response)
        }else if let Err(error) = _response{ return Err(error); }
        return Err(MailError::TCPConnectFailError)
    }

    fn _user(&mut self, account: String) -> Result<Response, MailError>{
        self.client.send("user ".to_owned() + &account)?;
        let _response = self.client.receive();
        if let Ok(response) = _response{
            return parse_response(response)
        }else if let Err(error) = _response{ return Err(error); }
        return Err(MailError::TCPConnectFailError)
    }

    fn _pass(&mut self, password: String) -> Result<Response, MailError>{
        self.client.send("pass ".to_owned() + &password)?;
        let _response = self.client.receive();
        if let Ok(response) = _response{
            return parse_response(response)
        }else if let Err(error) = _response{ return Err(error); }
        return Err(MailError::TCPConnectFailError)
    }

    fn _stat(&mut self) -> Result<Response, MailError>{
        self.client.send(String::from("stat"))?;
        let _response = self.client.receive();
        if let Ok(response) = _response{
            return parse_response(response)
        }else if let Err(error) = _response{ return Err(error); }
        return Err(MailError::TCPConnectFailError)
    }

    fn _list(&mut self, order_number: usize) -> Result<Response, MailError>{
        //list命令必须携带参数以和stat进行区别
        self.client.send(format!("list {}", order_number))?;
        let _response = self.client.receive();
        if let Ok(response) = _response{
            return parse_response(response);
        }else if let Err(error) = _response{ return Err(error); }
        return Err(MailError::TCPConnectFailError)
    }

    fn _top(&mut self, order_number: usize, top_line: usize) -> Result<Response, MailError>{
        self.client.send(format!("top {} {}", order_number, top_line))?;
        let mut counter = top_line;
        let mut result = String::new();
        let _response = self.client.receive();
        if let Ok(response) = _response{
            let response = parse_response(response);
            if let Ok(res) = response{
                if let Response::Ok(ok_res) = res{
                    result += &format!("{}\r\n", ok_res);
                }else if let Response::Err(error) = res{
                    return Ok(Response::Err(error));
                }
            }else if let Err(error) = response{
                return Err(error);
            }
        }else if let Err(error) = _response{
            return Err(error);
        }


        while counter > 0{
            let _response = self.client.receive();
            if let Ok(response) = _response{
                result += &response;
            }else if let Err(error) = _response{
                return Err(error);
            }
            counter -= 1;
        }

        return Ok(Response::Ok(result));
    }

    fn _retr(&mut self, order_number: usize) -> Result<Response, MailError>{
        self.client.send(format!("retr {}", order_number))?;
        let octets_info = self.client.receive();
        if let Ok(info) = octets_info{
            let parse_result = parse_response(info);
            if let Ok(response) = parse_result{
                if let Response::Ok(ok_response) = response{
                    let parse_result = parse_retr_octets_count(ok_response);
                    if let Ok(parse_result) = parse_result{
                        let mut line: String = String::new();
                        while line.len() <= parse_result{
                            let r = self.client.receive();
                            if let Ok(_line) = r{
                                line += &_line;
                            }else if let Err(error) = r{
                                return Err(error)
                            }
                        }
                        return Ok(Response::Ok(line));
                    }else if let Err(error) = parse_result{
                        return Err(error);
                    }
                }else if let Response::Err(err_response) = response{
                    return Ok(Response::Err(err_response));
                }
            }else if let Err(error) = parse_result{
                return Err(error);
            }
        }else if let Err(error) = octets_info{
            return Err(error);
        }
        return Err(MailError::NotHandledError);
    }

    fn _quit(&mut self) -> Result<Response, MailError>{
        self.client.send(String::from("quit"))?;
        let _response = self.client.receive();
        if let Ok(response) = _response{
            return parse_response(response);
        }else if let Err(error) = _response{ return Err(error); }
        return Err(MailError::TCPConnectFailError)
    }

    fn _dele(&mut self, order_number: usize) -> Result<Response, MailError>{
        self.client.send(format!("dele {}", order_number))?;
        let _response = self.client.receive();
        if let Ok(response) = _response{
            return parse_response(response);
        }else if let Err(error) = _response{ return Err(error); }
        return Err(MailError::TCPConnectFailError)
    }

    fn _rset(&mut self) -> Result<Response, MailError>{
        self.client.send(String::from("rset"))?;
        let _response = self.client.receive();
        if let Ok(response) = _response{
            return parse_response(response);
        }else if let Err(error) = _response{ return Err(error); }
        return Err(MailError::TCPConnectFailError)
    }


    //-----------------以下为业务函数------------------------------------------


    pub fn connect(&mut self){
        if let State::Init = self.state{
            if let Ok(response) = self._connect(){
                println!("客户端 <{}> 连接成功", self.name);
                self.print_debug(&response);
            }else{
                println!("客户端 <{}> 发起连接失败，请使用 -reset 进行重置", self.name);
            }
        }else{
            println!("客户端 <{}> 处于已连接状态，若要重置连接，请使用 -reset 命令", self.name);
        }
    }

    pub fn connection_test(&mut self){
        if let State::Init = self.state{
            println!("客户端 <{}> 尚未发起连接，请先尝试连接", self.name);
            return;
        }
        println!("正在测试客户端 <{}> 的连接状态，该操作耗时较长，请耐心等待", self.name);
        if let Ok(response) = self._noop(){
            println!("客户端 <{}> 的连接正常", self.name);
            self.print_debug(&response);
        }else{
            println!("客户端 <{}> 的连接已超时或丢失，请使用 -reset 进行重置", self.name);
        }
    }

    pub fn login(&mut self, account: String, password: String){

        match &self.state{
            State::Init => {
                println!("客户端 <{}> 尚未发起连接，请先尝试连接", self.name);
            }
            State::TCPConnected => {
                self.account = Some(account.clone());
                self.password = Some(password.clone());
                let user_result = self._user(account);
                if let Ok(response) = user_result{
                    self.print_debug(&response);
                    if let Response::Err(info) = response{
                        println!("客户端 <{}> 邮箱地址发送失败，服务器返回错误信息: <{}> ，请检查邮箱地址格式是否正确", self.name, info);
                    }
                }else if let Err(error) = user_result{
                    println!("客户端 <{}> 邮箱地址发送失败，错误类型: <{:?}>", self.name, error);
                }
                let pass_result = self._pass(password);
                if let Ok(response) = pass_result{
                    self.print_debug(&response);
                    if let Response::Err(info) = response{
                        println!("客户端 <{}> 授权码发送失败，服务器返回错误信息: <{}> ，请检查邮箱地址格式是否正确", self.name, info);
                    }else{
                        println!("客户端 <{}> 已登录为 <{}>", self.name, self.account.as_ref().unwrap());
                        self.state = State::Authorized;
                    }
                }else if let Err(error) = pass_result{
                    println!("客户端 <{}> 授权码发送失败，错误类型: <{:?}>", self.name, error);
                }
            }
            State::Authorized => {
                if let Some(account) = &self.account{
                    println!("客户端 <{}> 已登录为 <{}> ，若要重新登录请使用 -reset 进行重置", self.name, account);
                }else{
                    println!("未找到客户端 <{}> 的邮箱账号，请尝试使用 -reset 进行重置", self.name);
                }
            }
            _ => {
                println!("客户端 <{}> 登录时出现意外状态，请使用 -reset 进行重置", self.name);
            }
        }
    }

    pub fn status_query(&mut self){
        match &self.state{
            State::Init => { println!("客户端 <{}> 尚未发起连接，请先尝试连接", self.name); return; },
            State::TCPConnected => { println!("客户端 <{}> 尚未完成登录，请先尝试登录", self.name); return; },
            State::Authorized => {
                let status_result = self._stat();
                if let Ok(response) = status_result{
                    self.print_debug(&response);
                    if let Response::Ok(info) = response{
                        let parse_result = parse_status_response(info);
                        if let Ok(status) = parse_result{
                            println!("客户端 <{}> 的邮箱状态: 邮件总数:[{}] 邮件总字节数: [{}]", self.name, status.0, status.1);
                        }else if let Err(error) = parse_result{
                            println!("客户端 <{}> 邮箱状态获取失败，错误信息: {:?}", self.name, error);
                        }
                    }else if let Response::Err(info) = response{
                        println!("邮箱状态获取失败，错误信息: {}", info);
                    }
                }else if let Err(error) = status_result{
                    println!("客户端 <{}> 的连接已超时或丢失，请使用 -reset 进行重置", self.name);
                }
            }
            _ => {
                println!("客户端 <{}> 登录时出现意外状态，请使用 -reset 进行重置", self.name);
            }
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

    pub fn mail_info_short(&mut self, order_number: usize){
        match &self.state{
            State::Init => { println!("客户端 <{}> 尚未发起连接，请先尝试连接", self.name); return; },
            State::TCPConnected => { println!("客户端 <{}> 尚未完成登录，请先尝试登录", self.name); return; },
            State::Authorized => {
                let list_result = self._list(order_number);
                if let Ok(response) = list_result{
                    self.print_debug(&response);
                    if let Response::Ok(info) = response{
                        let parse_result = parse_status_response(info);
                        if let Ok(status) = parse_result{
                            println!("客户端 <{}> 的邮件信息: 第 <{}> 封邮件总字节数: [{}]", self.name, status.0, status.1);
                        }else if let Err(error) = parse_result{
                            println!("客户端 <{}> 邮箱状态获取失败，错误信息: {:?}", self.name, error);
                        }
                    }else if let Response::Err(info) = response{
                        println!("邮箱状态获取失败，错误信息: {}", info);
                    }
                }else if let Err(error) = list_result{
                    println!("客户端 <{}> 的连接已超时或丢失，请使用 -reset 进行重置", self.name);
                }
            }
            _ => {
                println!("客户端 <{}> 登录时出现意外状态，请使用 -reset 进行重置", self.name);
            }
        }
    }

    pub fn get_mail_by_order(&mut self, order_number: usize){
        match &self.state{
            State::Init => { println!("客户端 <{}> 尚未发起连接，请先尝试连接", self.name); return; },
            State::TCPConnected => { println!("客户端 <{}> 尚未完成登录，请先尝试登录", self.name); return; },
            State::Authorized => {
                let retr_result = self._retr(order_number);
                if let Ok(response) = retr_result{
                    self.print_debug(&response);
                    if let Response::Ok(res) = response{
                        let mail = parse_raw_mail(res);
                        if let Ok(pop3m) = mail{
                            println!("--- [{}] 号邮件详细内容---", order_number);
                            println!("发件人: {}", pop3m.from.unwrap_or_else(||{ (String::from("No Info"), String::from("No Info"))}).0);
                            println!("收件人: {}", pop3m.to.unwrap_or_else(||{ (String::from("No Info"), String::from("No Info"))}).0);
                            println!("邮件主题: {}", pop3m.subject.unwrap_or_else(||{ String::from("No Info")}));
                            println!("时间: {}", pop3m.time.unwrap_or_else(|| {String::from("No Info")}));
                            println!("邮件内容---");
                            println!("{}", pop3m.plain.unwrap_or_else(||{ String::from("No Info")}));

                        }else if let Err(error) = mail{
                            println!("客户端 <{}> 获取邮件详细内容失败，错误信息: {:?}", self.name, error);
                        }
                    }
                }else if let Err(error) = retr_result{
                    println!("客户端 <{}> 请求邮件内容出现问题，错误内容: [{:?}]", self.name, error);
                }
            }
            _ => {
                println!("客户端 <{}> 登录时出现意外状态，请使用 -reset 进行重置", self.name);
            }
        }
    }

    pub fn mail_list(&mut self){
        match &self.state{
            State::Init => { println!("客户端 <{}> 尚未发起连接，请先尝试连接", self.name); return; },
            State::TCPConnected => { println!("客户端 <{}> 尚未完成登录，请先尝试登录", self.name); return; },
            State::Authorized => {
                let stat = self._stat();
                let mut total: usize = 0;
                if let Ok(response) = stat{
                    self.print_debug(&response);
                    if let Response::Ok(ok_response) = response{
                        let parse_result = parse_status_response(ok_response);
                        if let Ok(ok_parse) = parse_result{
                            total = ok_parse.0;
                            println!("---------客户端 <{}> 的邮件列表: ---------", self.name);
                            println!("-> 邮件数: [{}]", total);
                        }else if let Err(err_parse) = parse_result{
                            println!("客户端 <{}> 请求邮件信息出现问题，错误内容: [{:?}]", self.name, err_parse);
                        }
                    }else if let Response::Err(err_response) = response{
                        println!("客户端 <{}> 请求邮件信息出现问题，错误内容: [{}]", self.name, err_response);
                    }
                }else if let Err(error) = stat{
                    println!("客户端 <{}> 请求邮件信息出现问题，错误信息: [{:?}]", self.name, error);
                }

                let mut counter: usize = 1;
                while counter <= total{
                    let mail_retr = self._retr(counter);
                    if let Ok(response) = mail_retr{
                        self.print_debug(&response);
                        if let Response::Ok(ok_response) = response{
                            let parse_result = parse_raw_mail(ok_response);
                            if let Ok(mail) = parse_result{
                                println!("({}) -> [{}] From [{}] to [{}]", counter, mail.subject.unwrap(), mail.from.unwrap().1, mail.to.unwrap().1);
                            }else if let Err(error) = parse_result{
                                println!("客户端 <{}> 对邮件进行解析时出现错误，错误内容: [{:?}]", self.name, error);
                            }
                        }else if let Response::Err(err_response) = response{
                            println!("客户端 <{}> 请求邮件信息出现问题，错误内容: [{:?}]", self.name, err_response);
                        }
                    }else if let Err(error) = mail_retr{
                        println!("客户端 <{}> 请求邮件列表出现问题，错误信息: [{:?}]({})", self.name, error, counter);
                    }
                    counter += 1;
                }
                println!();
            }
            _ => {
                println!("客户端 <{}> 登录时出现意外状态，请使用 -reset 进行重置", self.name);
            }
        }
    }

    pub fn quit(&mut self){
        let quit_result = self._quit();
        if let Ok(response) = quit_result{
            self.print_debug(&response);
            if let Response::Ok(ok_res) = response{
                println!("客户端 <{}> 已退出登录", self.name);
                self.client.shutdown();
                self.state = State::Init;
                if self.dele_list.len() > 0{
                    println!("以下序号的邮件将被删除:");
                    println!("-> {:?}", self.dele_list);
                }
                self.dele_list.clear();
            }else if let Response::Err(err_res) = response{
                println!("客户端 <{}> 退出失败，错误信息: <{}>", self.name, err_res);
            }
        }else if let Err(error) = quit_result{
            println!("客户端 <{}> 退出失败，错误信息: <{:?}>", self.name, error)
        }
    }

    pub fn delete(&mut self, order_number: usize){
        if self.dele_list.contains(&order_number){
            println!("客户端 <{}>的 [{}]号 邮件添加到删除列表中失败，错误信息: 邮件已在删除列表中", self.name, order_number);
            return;
        }
        let dele_result = self._dele(order_number);
        if let Ok(response) = dele_result{
            self.print_debug(&response);
            if let Response::Ok(ok_res) = response{
                println!("客户端 <{}> 已将 [{}]号 邮件添加到删除列表中，将会在退出时执行删除", self.name, order_number);
                self.dele_list.push(order_number);
            }else if let Response::Err(err_res) = response{
                println!("客户端 <{}>的 [{}]号 邮件添加到删除列表中失败，错误信息: <{}>", self.name, order_number, err_res)
            }
        }else if let Err(error) = dele_result{
            println!("客户端 <{}> 的 [{}]号 邮件添加到删除列表中失败，错误信息: <{:?}>", self.name, order_number, error);
        }
    }

    pub fn reset_delete(&mut self){
        let reset_result = self._rset();
        if let Ok(response) = reset_result{
            self.print_debug(&response);
            if let Response::Ok(ok_res) = response{
                println!("客户端 <{}> 的删除列表已重置", self.name);
                self.dele_list.clear();
            }else if let Response::Err(error) = response{
                println!("客户端 <{}> 删除列表重置失败，错误信息: {}", self.name, error)
            }
        }else if let Err(error) = reset_result{
            println!("客户端 <{}> 删除列表重置失败，错误信息: {:?}", self.name, error);
        }
    }

    pub fn display_delete_list(&mut self){
        if self.dele_list.len() > 0{
            println!("客户端 <{}> 的删除列表如下: ", self.name);
            println!("-> {:?}", self.dele_list);
        }else{
            println!("客户端 <{}> 的删除列表中无内容", self.name);
        }
        println!();
    }

    pub fn reset_client(&mut self){
        self.client.shutdown();
        self.state = State::Init;
        self.debug = false;
        self.dele_list = Vec::new();
    }

}

struct POP3Mail{
    time: Option<String>,
    from: Option<(String, String)>,
    to: Option<(String, String)>,
    subject: Option<String>,
    plain: Option<String>,
    html: Option<String>
}

impl POP3Mail{

    fn new() -> POP3Mail{
        POP3Mail{
            time: None,
            from: None,
            to: None,
            subject: None,
            plain: None,
            html: None
        }
    }

    fn check_integrity(&self) -> usize{
        if let Some(s) = &self.time{
            if let Some(s) = &self.from{
                if let Some(s) = &self.to{
                    if let Some(s) = &self.plain{
                        if let Some(s) = &self.html{
                            return 0;
                        }
                        return 5;
                    }
                    return 4;
                }
                return 3;
            }
            return 2;
        }
        return 1;
    }

    fn display(&self){
        println!("time: {:?}", self.time);
        println!("from: {:?}", self.from);
        println!("to: {:?}", self.to);
        println!("subject: {:?}", self.subject);
        println!("plain: {:?}", self.plain);
        println!("html: {:?}", self.html);
    }

    fn check_head_integrity(&self) -> usize{
        if let Some(_) = &self.time{
            if let Some(_) = &self.from{
                if let Some(_) = &self.to{
                    return 0;
                }
                return 3;
            }
            return 2;
        }
        return 1;
    }
}


fn parse_response(_response: String) -> Result<Response, MailError>{
    let response = _response.trim_end().to_string();
    let regex_ok = Regex::new(r"^\+OK (.*)$").unwrap();
    let regex_err = Regex::new(r"^-ERR (.*)$").unwrap();

    return if regex_ok.is_match(&response) {
        let mut ok_cap = regex_ok.captures(&response);
        let body = ok_cap.unwrap();
        Ok(Response::Ok(body[1].to_string().clone()))
    } else if regex_err.is_match(&response) {
        let mut err_cap = regex_err.captures(&response);
        let body = err_cap.unwrap();
        Ok(Response::Err(body[1].to_string().clone()))
    } else {
        Err(POP3ResponseParseError(response))
    }
}

fn parse_status_response(_response: String) -> Result<(usize, usize), MailError>{
    //将 stat/list命令的返回字符串解析为 (<邮件总数/邮件序号:int> <总字节数/指定邮件序号的字节数:int>)
    let response = _response.trim_end().to_string();
    let regex_status = Regex::new(r"^(\d+) (\d+)$").unwrap();
    return if regex_status.is_match(&response) {
        let contents = regex_status.captures(&response).unwrap();
        let param1 = contents[1].to_string().clone().trim().parse::<usize>();
        let param2 = contents[2].to_string().clone().trim().parse::<usize>();
        if let Ok(p1) = param1{
            if let Ok(p2) = param2{
                return Ok((p1, p2));
            }
        }
        Err(MailError::POP3StatusParseError(format!("邮箱状态解析失败: [{}]", response)))
    } else {
        Err(MailError::POP3StatusParseError(format!("邮箱状态解析失败: [{}]", response)))
    }
}

fn parse_retr_octets_count(_response: String) -> Result<usize, MailError>{
    let response = _response.trim_end().to_string();
    let octets_count_regex = Regex::new(r"^(\d+) octets$").unwrap();
    if octets_count_regex.is_match(&response){
        let mut cap = octets_count_regex.captures(&response);
        let _counter = cap.unwrap();
        let counter = _counter[1].to_string().trim().parse::<usize>().unwrap();
        return Ok(counter);
    }else{
        return Err(POP3ResponseParseError(response));
    }
    return Ok(0);
}

fn parse_raw_mail(raw: String) -> Result<POP3Mail, MailError>{
    let mut mail = POP3Mail::new();
    let lines = raw.lines();
    let time_regex = Regex::new(r"^Date: (.+)$").unwrap();
    let from_regex = Regex::new("^From: \"(.*)\" <(.+)>").unwrap();
    let to_regex = Regex::new("^To: \"(.*)\" <(.+)>").unwrap();
    let to1_regex = Regex::new("^To: (.*)").unwrap();
    let subject_regex = Regex::new("^Subject: (.*)").unwrap();
    let boundary_regex = Regex::new("^\tboundary=\"(.*)\"").unwrap();
    let mut boundary = String::from("");
    let mut lines1 = lines.clone();
    for line in lines{
        if time_regex.is_match(line){
            let mut cap = time_regex.captures(line).unwrap();
            mail.time.replace(cap[1].to_string());
        }else if from_regex.is_match(line){
            let mut cap = from_regex.captures(line).unwrap();
            mail.from.replace((cap[1].to_string(), cap[2].to_string()));
        }else if to_regex.is_match(line){
            let mut cap = to_regex.captures(line).unwrap();
            mail.to.replace((cap[1].to_string(), cap[2].to_string()));
        }else if subject_regex.is_match(line){
            let mut cap = subject_regex.captures(line).unwrap();
            mail.subject.replace(cap[1].to_string());
        }else if boundary_regex.is_match(line){
            let mut cap = boundary_regex.captures(line).unwrap();
            boundary = cap[1].to_string();
            break;
        }else if to1_regex.is_match(line){
            let mut cap = to1_regex.captures(line).unwrap();
            mail.to.replace(("".to_string(), cap[1].to_string()));
        }
    }

    if &boundary == ""{
        return Err(MailError::POP3MailIntegrityFailedError(7))// error code 7: No boundary sign found
    }
    let boundary_regex = Regex::new(&format!("(.*){}(.*)", boundary)).unwrap();


    let mut parsing_plain_text: bool = false;
    let mut parsing_html: bool = false;
    let mut plain_text_result = String::new();
    let mut html_result = String::new();
    let content_type_regex = Regex::new("Content-Type: (.+); charset=(.+)").unwrap();
    let content_transfer_encoding_regex = Regex::new("Content-Transfer-Encoding: (.+)").unwrap();
    let mut _7bit_mode = false;
    for line in lines1{
        if content_type_regex.is_match(line){
            let cap = content_type_regex.captures(line).unwrap();
            if &cap[2] == "GBK"{
                match & cap[1]{
                    "text/plain" => { parsing_plain_text = true; continue; }
                    "text/html" => { parsing_html = true; continue; }
                    _ => { return Err(MailError::POP3MailFormatNotSupportError(line.to_string())); }
                }
            }else { return Err(MailError::POP3MailFormatNotSupportError(line.to_string())); }
        }
        if content_transfer_encoding_regex.is_match(line){
            let cap = content_transfer_encoding_regex.captures(line).unwrap();
            match &cap[1]{
                "base64" => {

                }
                "7bit" => {
                    _7bit_mode = true;
                }
                _ => {
                    return Err(MailError::POP3MailFormatNotSupportError(line.to_string()));
                }
            }
            continue;
        }

        if boundary_regex.is_match(line){

            if parsing_html && !_7bit_mode{
                let error_copy = html_result.clone();
                let decode_result = base64::decode(error_copy.clone());
                if let Ok(ok_result) = decode_result{
                    let from_utf8_result = GBK.decode(&ok_result, DecoderTrap::Strict);
                    if let Ok(fu_result) = from_utf8_result{
                        mail.html.replace(fu_result);
                        parsing_plain_text = false;
                    }else if let Err(error) = from_utf8_result{
                        return Err(MailError::POP3MailDecodeError(error_copy.clone()));
                    }
                }else if let Err(error) = decode_result{
                    return Err(MailError::POP3MailDecodeError(error_copy.clone()));
                }
                continue;
            }else{
                mail.html.replace(html_result.clone());
                parsing_html = false;
            }
            if parsing_plain_text && !_7bit_mode{
                let error_copy = plain_text_result.clone();
                let decode_result = base64::decode(error_copy.clone());
                if let Ok(ok_result) = decode_result{
                    let from_utf8_result = GBK.decode(&ok_result, DecoderTrap::Strict);
                    if let Ok(fu_result) = from_utf8_result{
                        mail.plain.replace(fu_result);
                        parsing_plain_text = false;
                    }else if let Err(error) = from_utf8_result{
                        return Err(MailError::POP3MailDecodeError(error_copy.clone()));
                    }
                }else if let Err(error) = decode_result{
                    return Err(MailError::POP3MailDecodeError(error_copy.clone()));
                }
                continue;
            }else{
                mail.plain.replace(plain_text_result.clone());
                parsing_plain_text = false;
            }
        }

        if parsing_html{
            html_result = html_result + line;
            continue;
        }

        if parsing_plain_text{
            plain_text_result = plain_text_result + line;
            continue;
        }
    }

    if mail.check_integrity() != 0{
        return Err(MailError::POP3MailIntegrityFailedError(mail.check_integrity()));
    }

    return Ok(mail);
}

// fn parse_raw_mail_smtp(raw: String) -> Result<POP3Mail, MailError>{
//
// }
// fn parse_raw_mail_base64(raw: String) -> Result<POP3Mail, MailError>{
//
// }






pub fn run(name: String, account: String, password: String, pop3_host: String){
    let mut client = crate::pop3::Client::new(pop3_host, name);
    client.account.replace(account);
    client.password.replace(password);
    loop{
        let c = ||{
            return if let State::Authorized = client.state {
                "已登录".to_string()
            } else { "未登录".to_string() }
        };
        println!("正在运行: pop3客户端 <{}> ({})，使用 -help 获得帮助", client.name, c());
        let _input = util::input();
        let mut __input = _input.split_whitespace();
        if let Some(head) = __input.next(){
            match head{
                "-help" => {
                    println!("-login  自动发起连接并尝试登录");
                    println!("-test  测试连接状况");
                    println!("-delete [order number(int):邮件列表中序号]  将指定序号的邮件标记为删除");
                    println!("-list <delete:删除列表>/<mail:邮件列表(default)>");
                    println!("-detail [order number(int):邮件列表中序号]  显示指定序号邮件的详细内容");
                    println!("-reset <delete:删除列表>/<connection:连接信息(default)>  对指定内容进行重置");
                    println!("-debug  切换模式(普通/debug)");
                    println!("-quit <connection:tcp连接(default)>/<client:客户端>  执行quit命令，此时会执行删除操作");
                }
                "-login" => {
                    client.connect();
                    client.login(client.account.clone().unwrap(), client.password.clone().unwrap());
                }
                "-test" => {
                    client.connection_test();
                }
                "-delete" => {
                    if let Some(param) = __input.next(){
                        let _order_number = param.trim().parse::<usize>();
                        if let Ok(on) = _order_number{
                            client.delete(on);
                        }else { print_error(); }
                    }
                }
                "-list" => {
                    let k = __input.next();
                    if let Some("delete") = k{
                        client.display_delete_list();
                    }else if let Some("mail") = k{
                        client.mail_list();
                    }else if None == k{
                        client.mail_list();
                    }else{
                        print_error();
                    }
                }
                "-detail" => {
                    if let Some(_on) = __input.next(){
                        if let Ok(on) = _on.trim().parse::<usize>(){
                            client.get_mail_by_order(on);
                        }else{ print_error(); }
                    }else{
                        print_error();
                    }
                }
                "-reset" => {
                    let k = __input.next();
                    if let Some("delete") = k{
                        client.reset_delete();
                    }else if let Some("client") = k{
                        client.reset_client();
                    }else{
                        print_error();
                    }
                }
                "-debug" => {
                    client.debug();
                }
                "-quit" => {
                    let k = __input.next();
                    if let Some("connection") = k{
                        client.quit();
                    }else if let Some("client") = k{
                        client.quit();
                        break;
                    }else if None == k{
                        client.quit();
                    }else{
                        print_error();
                    }
                }
                _ => {
                    print_error();
                }
            }
        }else { print_error(); }
    }

}