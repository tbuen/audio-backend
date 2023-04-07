use crate::json::{ErrReq, Message, Rpc, RpcResult};
use data::Data;
pub use data::DirEntry;
pub use event::{Event, Reload};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod com;
mod data;
mod event;
mod json;

pub const VERSION: &str = env!("VERSION");

enum Command {
    Reload,
    Quit,
}

pub struct Backend {
    handle: Option<thread::JoinHandle<()>>,
    sender: Sender<Command>,
    data: Arc<Mutex<Data>>,
    tx: Option<Sender<Event>>,
    rx: Option<Receiver<Command>>,
}

impl Backend {
    pub fn new() -> (Self, Receiver<Event>) {
        let (sender, rx) = mpsc::channel();
        let (tx, receiver) = mpsc::channel();
        let data = Arc::new(Mutex::new(Data::new()));
        let backend = Self {
            handle: None,
            sender,
            data,
            tx: Some(tx),
            rx: Some(rx),
        };
        (backend, receiver)
    }

    pub fn start(&mut self) {
        if self.handle.is_none() {
            let data = self.data.clone();
            let tx = self.tx.take().unwrap();
            let rx = self.rx.take().unwrap();
            self.handle = Some(
                thread::Builder::new()
                    .name(String::from("audio:backend"))
                    .spawn(move || Self::thread(tx, rx, data))
                    .unwrap(),
            );
        }
    }

    pub fn shutdown(&mut self) {
        if self.handle.is_some() {
            self.sender.send(Command::Quit).unwrap();
            self.handle.take().unwrap().join().unwrap();
        }
    }

    pub fn current_dir(&self) -> String {
        let data = self.data.lock().unwrap();
        data.current_dir()
    }

    pub fn dir_enter(&self, dir: &str) {
        let mut data = self.data.lock().unwrap();
        data.dir_enter(dir);
    }

    pub fn dir_up(&self) {
        let mut data = self.data.lock().unwrap();
        data.dir_up();
    }

    pub fn dir_content(&self) -> Vec<DirEntry> {
        let data = self.data.lock().unwrap();
        data.dir_content()
    }

    pub fn reload(&self) {
        self.sender.send(Command::Reload).unwrap();
    }

    fn thread(tx: Sender<Event>, rx: Receiver<Command>, data: Arc<Mutex<Data>>) {
        let com = com::Com::new();
        let mut rpc = Rpc::new();

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Reload) => {
                    let mut data = data.lock().unwrap();
                    data.clear_file_list();
                    tx.send(Event::Reload(Reload::Start)).unwrap();
                    com.send(rpc.get_file_list(true));
                }
                Ok(Command::Quit) => {
                    println!("backend thread quit");
                    break;
                }
                Err(_) => {}
            }

            match com.recv_timeout(Duration::from_millis(10)) {
                Ok(com::Event::Connected) => {
                    println!("Connected!");
                    tx.send(Event::Connected).unwrap();
                    com.send(rpc.get_version());
                }
                Ok(com::Event::Disconnected) => {
                    println!("Disconnected!");
                    tx.send(Event::Disconnected).unwrap();
                }
                Ok(com::Event::Message(msg)) => {
                    println!("Message: {}", msg);
                    if let Some(m) = rpc.parse(&msg) {
                        println!("backend received message :-)");
                        Self::handle_message(m, &com, &mut rpc, &tx, &data);
                    }
                }
                Err(_) => {}
            }
        }

        println!("exit backend thread");
    }

    fn handle_message(
        msg: Message,
        com: &com::Com,
        rpc: &mut Rpc,
        tx: &Sender<Event>,
        data: &Arc<Mutex<Data>>,
    ) {
        match msg {
            Message::Response(r) => match r {
                Ok(r) => match r {
                    RpcResult::Version(ver) => {
                        let evt = Event::Version(event::Version {
                            project: String::from(&ver.project),
                            version: String::from(&ver.version),
                            esp_idf: String::from(&ver.esp_idf),
                        });
                        tx.send(evt).unwrap();
                    }
                    RpcResult::FileList(lst) => {
                        let mut data = data.lock().unwrap();
                        if lst.first {
                            data.clear_file_list();
                        }
                        data.append_file_list(lst.files);
                        if lst.last {
                            match data.get_unsynced_file() {
                                Some(f) => {
                                    tx.send(Event::Reload(Reload::Step)).unwrap();
                                    com.send(rpc.get_file_info(f));
                                }
                                None => {
                                    tx.send(Event::Reload(Reload::Stop)).unwrap();
                                }
                            }
                        } else {
                            tx.send(Event::Reload(Reload::Step)).unwrap();
                            com.send(rpc.get_file_list(false));
                        }
                    }
                    RpcResult::FileInfo(info) => {
                        let mut data = data.lock().unwrap();
                        data.set_file_info(info);
                        match data.get_unsynced_file() {
                            Some(f) => {
                                tx.send(Event::Reload(Reload::Step)).unwrap();
                                com.send(rpc.get_file_info(f));
                            }
                            None => {
                                tx.send(Event::Reload(Reload::Stop)).unwrap();
                            }
                        }
                    }
                },
                Err(e) => {
                    tx.send(Event::Error(format!(
                        "{}: {} ({})",
                        e.method, e.message, e.code
                    )))
                    .unwrap();
                    match e.request {
                        ErrReq::Version => {}
                        ErrReq::FileList => {
                            tx.send(Event::Reload(Reload::Stop)).unwrap();
                        }
                        ErrReq::FileInfo => {
                            tx.send(Event::Reload(Reload::Stop)).unwrap();
                        }
                        _ => {}
                    }
                }
            },
            //Message::Notification => {}
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
