use crate::com::{Com, Event};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod com;

pub const VERSION: &str = env!("VERSION");

//#[derive(Default, Debug, Deserialize)]
#[derive(Debug)]
pub struct Info {
    pub project: String,
    pub version: String,
    //   #[serde(rename = "esp-idf")]
    pub esp_idf: String,
}

enum Command {
    Idle,
    Quit,
}

pub enum Response {
    Connected,
    Disconnected,
    Info(Info),
}

pub struct Backend {
    handle: Option<thread::JoinHandle<()>>,
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<Response>,
}

impl Backend {
    pub fn new() -> Self {
        let (thread_tx, rx) = mpsc::channel();
        let (tx, thread_rx) = mpsc::channel();
        Self {
            handle: Some(
                thread::Builder::new()
                    .name(String::from("audio:backend"))
                    .spawn(move || Self::thread(thread_tx, thread_rx))
                    .unwrap(),
            ),
            tx,
            rx,
        }
    }

    pub fn receive(&self) -> Option<Response> {
        self.tx.send(Command::Idle).unwrap();
        self.rx.try_recv().ok()
    }

    fn thread(tx: mpsc::Sender<Response>, rx: mpsc::Receiver<Command>) {
        let com = Com::new();

        loop {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(Command::Idle) => {
                    println!("backend thread idle");
                }
                Ok(Command::Quit) => {
                    println!("backend thread quit");
                    break;
                }
                Err(_) => {}
            }

            match com.recv_timeout(Duration::from_millis(10)) {
                Ok(Event::Connected) => {
                    println!("Connected!");
                    tx.send(Response::Connected).unwrap();
                    com.send(String::from(r#"{"jsonrpc":"2.0","method":"bla","id":1}"#));
                }
                Ok(Event::Disconnected) => {
                    println!("Disconnected!");
                    tx.send(Response::Disconnected).unwrap();
                }
                Ok(Event::Message(msg)) => {
                    println!("Message: {}", msg);
                    tx.send(Response::Info(Info {
                        project: String::from("boomi"),
                        version: String::from("10.1"),
                        esp_idf: String::from("neu"),
                    }))
                    .unwrap();
                }
                Err(_) => {}
            }
        }

        println!("exit backend thread");
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
