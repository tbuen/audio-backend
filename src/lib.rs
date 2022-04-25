use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

pub const VERSION: &str = env!("VERSION");

#[derive(Default, Debug, Deserialize)]
pub struct Info {
    pub project: String,
    pub version: String,
    #[serde(rename = "esp-idf")]
    pub esp_idf: String,
}

pub enum Response {
    Info(Info),
}

pub struct Backend {
    handle: Option<thread::JoinHandle<()>>,
    recv: Receiver<Response>,
}

impl Backend {
    pub fn new() -> Backend {
        let (send, recv) = channel();
        Backend {
            recv,
            handle: Some(thread::spawn(|| backend_thread(send))),
        }
    }

    // TODO maybe do this with Drop trait?
    pub fn stop(&mut self) {
        if self.handle.is_some() {
            self.handle.take().unwrap().join().unwrap();
        }
    }

    pub fn receive(&self) -> Option<Response> {
        match self.recv.try_recv() {
            Ok(resp) => return Some(resp),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                println!("Thread disconnected!")
            }
        }
        None
    }
}

fn backend_thread(send: Sender<Response>) {
    let client = Client::builder()
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
