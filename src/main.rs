#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::IpAddr;
use std::process::ExitCode;

mod capture;
mod packet;
mod session;
mod ui;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let result = match args.next().as_deref() {
        None => session::serve(),
        Some("send") => match args.next() {
            None => session::send(IpAddr::from([127, 0, 0, 1]), Some(50)),
            Some(host) => match host.parse::<IpAddr>() {
                Ok(ip) if args.next().is_none() => session::send(ip, Some(50)),
                Ok(_) => Err("send takes only an ip".into()),
                Err(_) => Err(format!("send expects an ip, got {host}").into()),
            },
        },
        Some("capture") if args.next().is_none() => capture::capture(),
        Some("recv") if args.next().is_none() => session::recv(),
        Some("recv") => Err("recv takes no arguments".into()),
        Some(other) => Err(format!("unknown command {other}. use send or recv").into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            if std::env::args().nth(1).is_none() {
                ui::alert(&err.to_string());
            }
            ExitCode::FAILURE
        }
    }
}
