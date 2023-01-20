//use reqwest::blocking::Client;
//use reqwest::StatusCode;
//use serde::Deserialize;
use crate::com::Client;
use std::net::SocketAddrV4;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{spawn, JoinHandle};
//use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

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

#[derive(Debug)]
pub enum Command {
    Idle,
    Quit,
    Info,
    Mdns(SocketAddrV4),
}

pub enum Response {
    Info(Info),
}

pub struct Backend {
    handle: Option<JoinHandle<()>>,
    tx: UnboundedSender<Command>,
    _rx: Receiver<Response>,
}

impl Backend {
    pub fn new() -> Backend {
        let (thread_tx, rx) = channel();
        let (tx, thread_rx) = unbounded_channel();
        Backend {
            handle: Some(spawn(|| Self::thread(thread_tx, thread_rx))),
            tx,
            _rx: rx,
        }
    }

    /*pub fn send(&self, cmd: Command) {
        self.tx.send(cmd);
    }*/

    pub fn receive(&self) -> Option<Response> {
        self.tx.send(Command::Idle).unwrap();
        /*match self.rx.try_recv() {
            Ok(resp) => return Some(resp),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                println!("Thread disconnected!")
            }
        }*/
        None
    }

    #[tokio::main(flavor = "current_thread")]
    async fn thread(_tx: Sender<Response>, mut rx: UnboundedReceiver<Command>) {
        /*let client = Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        loop {
            let mut info: Info = Default::default();
            let resp = client.get("http://192.168.4.1/info").send();
            match resp {
                Ok(resp) => {
                    println!("Response: {:?}", resp);
                    if resp.status() == StatusCode::OK {
                        if let Ok(data) = resp.json() {
                            info = data
                        }
                    }
                }
                Err(error) => println!("{:?}", error),
            };
            send.send(Response::Info(info)).unwrap();
            thread::sleep(Duration::from_secs(5));
        }*/

        let mut client = Client::new();

        loop {
            tokio::select! {
                Some(resp) = rx.recv() => {
                    println!("backend thread received {:?} from gui", resp);
                    match resp {
                        Command::Quit => {client.shutdown(); rx.close();},
                        _ => {},
                    }
                },
                Some(resp) = client.recv() => {
                    println!("backend thread received {} from com", resp);
                }
                else => {
                    break;
                }
            }
        }

        println!("exit backend thread");
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.tx.send(Command::Quit).unwrap();
            self.handle.take().unwrap().join().unwrap();
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
