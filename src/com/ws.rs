use futures_util::StreamExt;
use std::net::SocketAddrV4;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
//use tokio::time ::{sleep,Duration};

pub struct Ws {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Ws {
    pub async fn connect(socket: SocketAddrV4) -> Option<Ws> {
        println!("try to connect");
        //match connect_async("ws://192.168.188.55:80/ws").await {
        match connect_async(format!("ws://{}:{}/ws", socket.ip(), socket.port())).await {
            Ok((s, _r)) => Some(Ws { stream: s }),
            Err(e) => {
                println!("ws connect failed! {}", e);
                None
            }
        }
    }

    pub async fn shutdown(&mut self) {
        self.stream.close(None).await.unwrap();
    }

    pub async fn recv(&mut self) -> Option<String> {
        match self.stream.next().await {
            Some(Ok(msg)) => {
                println!("ws received msg: {:?}", msg);
                Some(String::from("bla"))
            }
            Some(Err(e)) => {
                println!("ws received error: {}", e);
                None
            }
            None => {
                println!("ws received nothing");
                None
            }
        }
    }
}
