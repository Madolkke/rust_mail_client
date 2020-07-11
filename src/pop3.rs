use crate::tcp;
use crate::pop3::State::TCPConnected;
use crate::util;
use crate::util::MailError;
use regex::Regex;
use crate::util::MailError::POP3ResponseParseError;



#[derive(Debug)]
pub enum Response{
    Ok(String),
    Err(String)
}

pub enum State{
    Init, TCPConnected, Authorized
}

pub struct Client{
    client: tcp::Client,
    state: State,
    name: String,
    account: Option<String>,
    password: Option<String>,
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
        let _response1 = self.client.receive();
        let _response2 = self.client.receive();
        println!("{:?}", _response2);
        if let Ok(response) = _response1{
            return parse_response(response);
        }else if let Err(error) = _response1{ return Err(error); }
        return Err(MailError::TCPConnectFailError)
    }

    fn _retr(&mut self, order_number: usize) -> Result<Response, MailError>{
        self.client.send(format!("retr {}", order_number))?;
        let mut octets_counter: usize = 0;
        let octets_info = self.client.receive();
        if let Ok(info) = octets_info{
            let recorder = info.len();
            let parse_result = parse_response(info);
            if let Ok(response) = parse_result{
                if let Response::Ok(ok_response) = response{
                    let parse_result = parse_retr_octets_count(ok_response);
                    if let Ok(parse_result) = parse_result{

                        octets_counter += recorder;
                        let mut line: String = String::new();
                        while true{
                            let r = self.client.receive();
                            if let Ok(_line) = r{
                                line = _line;
                                println!("{}", line);
                                octets_counter += line.len();
                            }else if let Err(error) = r{
                                return Err(error)
                            }
                        }



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

    pub fn list(&mut self, order_number: usize){
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

pub(crate) fn run(){

    let client_list: Vec<crate::pop3::Client> = Vec::new();
    let mut test_client = crate::pop3::Client::new(String::from("pop.126.com"), String::from("test"));
    test_client.debug();
    test_client.connect();
    test_client.login(String::from("madolkke@126.com"), String::from("MVHEFVZAMGXLJXBR"));
    test_client._retr(3);

}