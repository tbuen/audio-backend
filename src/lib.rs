use crate::json::{Message, Rpc, RpcResult};
use data::{Data, File};
pub use event::Event;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod com;
mod data;
mod event;
mod json;

pub const VERSION: &str = env!("VERSION");

enum Command {
    Quit,
}

pub struct Backend {
    handle: Option<thread::JoinHandle<()>>,
    tx: Sender<Command>,
    data: Arc<Mutex<Data>>,
}

impl Backend {
    pub fn new() -> (Self, Receiver<Event>) {
        let (tx, rx_thread) = channel();
        let (tx_thread, rx) = channel();
        let data = Arc::new(Mutex::new(Data::new()));
        let data_thread = data.clone();
        (
            Self {
                handle: Some(
                    thread::Builder::new()
                        .name(String::from("audio:backend"))
                        .spawn(move || Self::thread(tx_thread, rx_thread, data_thread))
                        .unwrap(),
                ),
                tx,
                data,
            },
            rx,
        )
    }

    pub fn current_dir(&self) -> String {
        let data = self.data.lock().unwrap();
        data.current_dir()
    }

    pub fn file_list(&self) -> Vec<File> {
        let data = self.data.lock().unwrap();
        data.file_list()
    }

    fn thread(tx: Sender<Event>, rx: Receiver<Command>, data: Arc<Mutex<Data>>) {
        let com = com::Com::new();
        let mut rpc = Rpc::new();

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
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
                    com.send(rpc.get_file_list());
                }
                Ok(com::Event::Disconnected) => {
                    println!("Disconnected!");
                    tx.send(Event::Disconnected).unwrap();
                }
                Ok(com::Event::Message(msg)) => {
                    println!("Message: {}", msg);
                    if let Some(m) = rpc.parse(&msg) {
                        println!("backend received message :-)");
                        Self::handle_message(m, &tx, &data);
                    }
                }
                Err(_) => {}
            }
        }

        println!("exit backend thread");
    }

    fn handle_message(msg: Message, tx: &Sender<Event>, data: &Arc<Mutex<Data>>) {
        match msg {
            Message::Response(r) => match r {
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
                    data.set_file_list(lst);
                    tx.send(Event::Synchronized).unwrap();
                }
            },
            //Message::Notification => {}
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.tx.send(Command::Quit).unwrap();
        self.handle.take().unwrap().join().unwrap();
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
