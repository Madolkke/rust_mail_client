mod tcp;
mod smtp;
mod util;
mod pop3;

extern crate regex;
extern crate chrono;

use std::net::{TcpStream, Shutdown};
use std::io::{Write, Read, BufReader, BufRead};
use util::print_error;
use chrono::prelude::*;
use crate::smtp::SMTPMail;
//pop3:MVHEFVZAMGXLJXBR

fn main() {

    let mut mail_list: Vec<SMTPMail> = Vec::new();
    let mut smtp_host: Option<String> = None;
    let mut pop3_host: Option<String> = None;
    let mut password: Option<String> = None;
    let mut account: Option<String> = None;
    let mut name: Option<String> = None;

    loop {
        println!("正在使用: 邮箱客户端主菜单，使用 -help 获得帮助");
        let _input = util::input();
        let mut __input = _input.split_whitespace();
        if let Some(head) = __input.next(){
            match head{
                "-help" => {
                    println!("-pop3  进入pop3收件客户端");
                    println!("-smtp  进入smtp发件客户端");
                    println!("-editor  进入邮件编辑器");
                    println!("-list  查看已保存邮件列表");
                    println!("-config  配置用户信息");
                    println!("-quit  退出邮件客户端");
                }
                "-pop3" => {
                    if let (Some(n), Some(acc), Some(pass), Some(p3h)) = (&name, &account, &password, &pop3_host){
                        pop3::run(n.clone(), acc.clone(), pass.clone(), p3h.clone());
                    }else{
                        println!("配置信息不完整，请配置后重试");
                    }
                }
                "-smtp" => {
                    if let (Some(n), Some(acc), Some(pass), Some(sh)) = (&name, &account, &password, &smtp_host){
                        smtp::run(n.clone(), acc.clone(), pass.clone(), sh.clone(), &mut mail_list);
                    }else{
                        println!("配置信息不完整，请配置后重试");
                    }
                }
                "-editor" => {
                    util::MailEditor::run(&mut mail_list);
                }
                "-list" => {
                    if mail_list.len() == 0{
                        println!("无已保存的邮件，可使用编辑器创建新的邮件"); continue;
                    }
                    println!("---已保存的邮件列表---");
                    let mut counter: usize = 0;
                    for mail in &mail_list{
                        counter += 1;
                        if let Some(sub) = &mail.subject{
                            println!("[{}] {}", counter, sub);
                        }else{
                            println!("[{}] * Mail No Subject *", counter);
                        }
                    }
                    println!();
                }
                "-config" => {
                    println!("---当前配置信息---");
                    println!("pop3: {}", pop3_host.clone().unwrap_or_else(||String::from("No info")));
                    println!("smtp: {}", smtp_host.clone().unwrap_or_else(||String::from("No info")));
                    println!("account: {}", account.clone().unwrap_or_else(||String::from("No info")));
                    println!("password: {}", password.clone().unwrap_or_else(||String::from("No info")));
                    println!("name: {}", name.clone().unwrap_or_else(||String::from("No info")));
                    println!("已配置的用户信息将会被覆盖，确认要进行配置吗？(yes/no)");
                    let _input = util::input();
                    if _input == "yes"{
                        println!("请输入 pop3 主机地址");
                        pop3_host.replace(util::input());
                        println!("请输入 smtp 主机地址");
                        smtp_host.replace(util::input());
                        println!("请输入 邮箱账号");
                        account.replace(util::input());
                        println!("请输入 授权码");
                        password.replace(util::input());
                        println!("请输入 本地主机名");
                        name.replace(util::input());
                        println!("配置完成");
                        println!();
                    }else if _input == "no"{ continue; } else { print_error(); continue;}
                }
                "-quit" => {
                    println!("确定要退出邮箱客户端吗？(yes/no)");
                    let _input = util::input();
                    if _input == "yes"{ break; }else if _input == "no"{ continue; } else { print_error(); continue;}
                }
                _ => { print_error(); }
            }
        }
    }

}
