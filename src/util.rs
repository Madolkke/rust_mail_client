use trust_dns_resolver::Resolver;
use trust_dns_resolver::config;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use std::net::{IpAddr, Ipv4Addr};
use crate::smtp::SMTPMail;
use trust_dns_resolver::proto::rr::record_data::RData::OPT;
use regex::Regex;

#[derive(Debug)]
pub enum MailError{
    TCPFromUTF8Error, TCPWriteError, TCPNoConnectionError, TCPConnectFailError,

    NotHandledError,

    POP3ResponseParseError(String), POP3StatusParseError(String), POP3MailIntegrityFailedError(usize),
    POP3MailDecodeError(String), POP3MailFormatNotSupportError(String),

    SMTPResponseParseError(String), SMTPResponseNotErrorCodeError(String),
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

pub fn input() -> String{
    let mut line = String::new();
    std::io::stdin().read_line(&mut line);
    line.trim_end().to_string()
}

pub fn print_error(){
    println!("不支持的命令格式，请重新输入");
}

pub struct MailEditor{
    current_order: usize,
    current_saved: bool,
    lines: Vec<String>,
    saved_mails: Vec<SMTPMail>,
    current_from: Option<String>,
    current_to: Option<String>,
    current_subject: Option<String>,
    current_content: Option<Vec<String>>
}

impl MailEditor {
    pub fn run(mail_list: &mut Vec<SMTPMail>){
        let mut editor = MailEditor{
            current_order: 0,
            current_saved: false,
            current_from: None,
            current_to: None,
            current_subject: None,
            current_content: None,
            lines: vec![],
            saved_mails: mail_list.clone(),
        };
        loop{
            println!("正在使用: 邮件编辑器<{}号邮件>，使用 -help 获取帮助", editor.current_order);
            let _input = input();
            let mut __input = _input.split_whitespace();
            if let Some(head) = __input.next(){
                match head{
                    "-help" => {
                        println!("-edit <from:邮件发送者>/<to:邮件接收者>/<subject:邮件主题>/<content:邮件内容 <append:添加行>/<line:修改行>/<remove:删除行>/<all:重新编辑所有内容>>  编辑邮件内容");
                        println!("-check <format:邮件格式信息>/<integrity:邮件完整性>  对邮件进行正确性检测");
                        println!("-display <all:所有内容>/<subject:邮件主题>/<from:邮件发送者>/<to:邮件接收者>/<content:邮件内容>  显示编辑中的邮件内容");
                        println!("-list  显示已保存邮件列表");
                        println!("-save  保存当前邮件");
                        println!("-take [order number(int):已保存邮件列表中序号]/<new:创建新邮件>  切换编辑器至指定邮件");
                        println!("-delete [order number(int):已保存邮件列表中序号]/<current:删除当前邮件>  删除指定邮件");
                        println!("-quit 退出邮件编辑器");
                        println!();
                    }
                    "-edit" => {
                        if let Some(param1) = __input.next(){
                            if editor.current_order == 0{
                                println!("未找到正在编辑的邮件，请使用 -take 创建或加载邮件"); continue;
                            }
                            match param1{
                                "from" => {
                                    println!("请输入 [{}] 号邮件的发送邮箱地址:", editor.current_order);
                                    let new_from = input();
                                    editor.current_from.replace(new_from);
                                    editor.current_saved = false;
                                    println!("[{}] 号邮件的发送邮箱地址已修改为: [{}]", editor.current_order, editor.current_from.clone().unwrap());
                                }
                                "to" => {
                                    println!("请输入 [{}] 号邮件的接收邮箱地址:", editor.current_order);
                                    let input = input();
                                    editor.current_to.replace(input);
                                    editor.current_saved = false;
                                    println!("[{}] 号邮件的接收邮箱地址已修改为: [{}]", editor.current_order, editor.current_to.clone().unwrap());
                                }
                                "subject" => {
                                    println!("请输入 [{}] 号邮件的主题:", editor.current_order);
                                    let input = input();
                                    editor.current_subject.replace(input);
                                    editor.current_saved = false;
                                    println!("[{}] 号邮件的主题已修改为: [{}]", editor.current_order, editor.current_subject.clone().unwrap());
                                }
                                "content" => {
                                    if let Some(param2) = __input.next(){
                                        match param2{
                                            "append" => {
                                                println!("请按行输入邮件内容，若要结束输入请键入 -exit");
                                                loop{
                                                    let line = input();
                                                    if line.as_str() == "-exit"{ break; }
                                                    editor.lines.push(line);
                                                }
                                                editor.current_content.replace(editor.lines.clone());
                                                editor.current_saved = false;
                                                println!("邮件内容已更新，可使用 -display 进行查看");
                                                println!();
                                            }
                                            "line" => {
                                                println!("请输入要修改的行序号:");
                                                let _on = input();
                                                let __on = _on.trim().parse::<usize>();
                                                if let Ok(on) = __on{
                                                    if on > editor.lines.len(){
                                                        print_error(); continue;
                                                    }else{
                                                        println!("请输入修改后的行内容:");
                                                        let new_line = input();
                                                        editor.lines[on - 1] = new_line;
                                                        editor.current_content.replace(editor.lines.clone());
                                                        editor.current_saved = false;
                                                        println!("邮件内容已更新，可使用 -display 进行查看");
                                                        println!();
                                                    }
                                                }else { print_error(); }
                                            }
                                            "remove" => {
                                                println!("请输入要删除的行序号:");
                                                let _on = input();
                                                let __on = _on.trim().parse::<usize>();
                                                if let Ok(on) = __on{
                                                    if on > editor.lines.len(){
                                                        print_error(); continue;
                                                    }else{
                                                        editor.lines.remove(on - 1);
                                                        editor.current_saved = false;
                                                        editor.current_content.replace(editor.lines.clone());
                                                        println!("邮件内容已更新，可使用 -display 进行查看");
                                                        println!();
                                                    }
                                                }else { print_error(); }
                                            }
                                            "all" => {
                                                editor.lines.clear();
                                                println!("请按行输入邮件内容，若要结束输入请键入 -exit");
                                                loop{
                                                    let line = input();
                                                    if line.as_str() == "-exit"{ break; }
                                                    editor.lines.push(line);
                                                }
                                                editor.current_content.replace(editor.lines.clone());
                                                editor.current_saved = false;
                                                println!("邮件内容已更新，可使用 -display 进行查看");
                                                println!();
                                            }
                                            _ => { print_error(); }
                                        }
                                    }else { print_error(); }
                                }
                                _ => { print_error(); }
                            }
                        }else{ print_error(); }
                    }
                    "-check" => {
                        print_error();
                    }
                    "-display" => {
                        if let Some(param1) = __input.next(){
                            match param1{
                                "from" => {
                                    if let Some(from) = &editor.current_from{
                                        println!("[{}] 号邮件发送邮箱地址: [{}]", editor.current_order, from);
                                    }else{
                                        println!("[{}] 号邮件无已保存的发送邮箱地址", editor.current_order);
                                    }
                                }
                                "to" => {
                                    if let Some(to) = &editor.current_to{
                                        println!("[{}] 号邮件接收邮箱地址: [{}]", editor.current_order, to);
                                    }else{
                                        println!("[{}] 号邮件无已保存的接收邮箱地址", editor.current_order);
                                    }
                                }
                                "subject" => {
                                    if let Some(subject) = &editor.current_subject{
                                        println!("[{}] 号邮件主题: [{}]", editor.current_order, subject);
                                    }else{
                                        println!("[{}] 号邮件无已保存的邮件主题", editor.current_order);
                                    }
                                }
                                "content" => {
                                    if let Some(content) = &editor.current_content{
                                        println!("---[{}] 号邮件内容---", editor.current_order);
                                        let mut counter: usize = 0;
                                        for line in content{
                                            counter += 1;
                                            println!("[{}] {}", counter, line);
                                        }
                                        println!();
                                    }else{
                                        println!("[{}] 号邮件无已保存的邮件内容", editor.current_order);
                                    }
                                }
                                "all" => {
                                    if let Some(from) = &editor.current_from{
                                        println!("[{}] 号邮件发送邮箱地址: [{}]", editor.current_order, from);
                                    }else{
                                        println!("[{}] 号邮件无已保存的发送邮箱地址", editor.current_order);
                                    }
                                    if let Some(to) = &editor.current_to{
                                        println!("[{}] 号邮件接收邮箱地址: [{}]", editor.current_order, to);
                                    }else{
                                        println!("[{}] 号邮件无已保存的接收邮箱地址", editor.current_order);
                                    }
                                    if let Some(subject) = &editor.current_subject{
                                        println!("[{}] 号邮件主题: [{}]", editor.current_order, subject);
                                    }else{
                                        println!("[{}] 号邮件无已保存的邮件主题", editor.current_order);
                                    }
                                    if let Some(content) = &editor.current_content{
                                        println!("---[{}] 号邮件内容---", editor.current_order);
                                        let mut counter: usize = 0;
                                        for line in content{
                                            counter += 1;
                                            println!("[{}] {}", counter, line);
                                        }
                                        println!();
                                    }else{
                                        println!("[{}] 号邮件无已保存的邮件内容", editor.current_order);
                                    }
                                }
                                _ => { print_error(); }
                            }
                        }else { print_error(); }
                    }
                    "-list" => {
                        if editor.saved_mails.len() == 0{
                           println!("编辑器中未发现已保存的邮件");
                        }else{
                            println!("---编辑器中已保存的邮件列表---");
                            let mut counter: usize = 0;
                            for mail in &editor.saved_mails{
                                if let Some(sub) = &mail.subject{
                                    println!("[{}]  {}", counter + 1, sub);
                                }else{
                                    println!("[{}]  * Mail No Subject *", counter + 1);
                                }
                                counter += 1;
                            }
                            println!();
                        }
                    }
                    "-save" => {
                        let pointer = editor.saved_mails.get_mut(editor.current_order - 1).unwrap();
                        pointer.subject = editor.current_subject.clone();
                        pointer.from = editor.current_from.clone();
                        pointer.to = editor.current_to.clone();
                        pointer.content = editor.current_content.clone();
                        editor.current_saved = true;
                        println!("[{}] 号邮件已保存", editor.current_order);
                    }
                    "-take" => {
                        if let Some(param1) = __input.next(){
                            if !editor.current_saved && (editor.current_order != 0){
                                println!("当前邮件尚未保存，请保存后重试或使用 -delete current 删除当前邮件"); continue;
                            }
                            if param1 == "new"{
                                editor.saved_mails.push(SMTPMail::new());
                                editor.current_order = editor.saved_mails.len();
                                editor.current_saved = false;
                                editor.current_from = None;
                                editor.current_to = None;
                                editor.current_subject = None;
                                editor.current_content = None;
                                println!("已创建新邮件，序号为 [{}]", editor.current_order); continue;
                            }
                            if let Ok(order_number) = param1.trim().parse::<usize>(){
                                if order_number > editor.saved_mails.len(){
                                    println!("未找到序号为 [{}] 的邮件", order_number);
                                }else{
                                    editor.current_order = order_number;
                                    let copy = editor.saved_mails.get(order_number).unwrap();
                                    editor.current_content = copy.content.clone();
                                    editor.current_subject = copy.subject.clone();
                                    editor.current_from = copy.from.clone();
                                    editor.current_to = copy.to.clone();
                                    editor.current_saved = false;
                                    println!("已读取序号为 [{}] 的邮件", order_number);
                                }
                            }else { print_error(); }
                        }else { print_error(); }
                    }
                    "-delete" => {
                        if let Some(param1) = __input.next(){
                            if editor.current_order == 1 { println!("当前无可删除邮件"); continue; }
                            if param1 == "current"{
                                editor.clear_current();
                                editor.saved_mails.remove(editor.current_order - 1);
                                println!("已删除当前邮件"); continue;
                            }else if let Ok(order_number) = param1.trim().parse::<usize>(){
                                editor.saved_mails.remove(order_number - 1);
                                if editor.current_order == order_number{
                                    editor.clear_current();
                                    println!("已删除当前邮件"); continue;
                                }else if editor.current_order > order_number{
                                    editor.current_order -= 1;
                                    println!("已删除 [{}] 号邮件", order_number);
                                }else if editor.current_order < order_number{
                                    editor.current_order += 1;
                                    println!("已删除 [{}] 号邮件", order_number);
                                }
                            }else { print_error(); }
                        }else { print_error(); }
                    }
                    "-quit" => {
                        if !editor.current_saved{
                            println!("当前邮件尚未保存，请保存后再尝试退出"); continue;
                        }
                        break;
                    }
                    _ => { print_error(); }
                }
            }else{ print_error(); }
        }
        mail_list.clone_from(&editor.saved_mails);
        println!("已退出邮件编辑器");
    }

    pub fn clear_current(&mut self){
        self.current_content = None;
        self.current_saved = false;
        self.current_order = 0;
        self.current_subject = None;
        self.current_to = None;
        self.current_from = None;
        self.lines.clear();
    }
}