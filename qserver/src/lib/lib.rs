use bytes::BytesMut;
use bytes::BufMut;
use flume::Sender;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    runtime::{self, Runtime},
};

//The server architecture must be game agnostic. That is it must only provide
//network communication functionalty and systems. The Quniverse and QSwitch will use these
//systems and functionalutity to "make a game"

pub struct QServer {
    rt: Runtime,
}

impl QServer {
    pub fn new<T: ToSocketAddrs>(bindpoint: T) -> QServer {
        let rt = runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(Self::manage_server(bindpoint));

        QServer { rt }
    }
    async fn manage_server<T: ToSocketAddrs>(bindpoint: T) {
        let server = TcpListener::bind(bindpoint)
            .await
            .expect("Could not bind server");

        loop {
            let (mut socket, connection) = server
                .accept()
                .await
                .expect("Could not accept new connection");
            println!("Accepted connection from {:?}", connection);

            tokio::spawn(async move {
                let mut data = BytesMut::with_capacity(10000000);
                {
                let data = &mut data;
                let mut limit = data.limit(100003);
                loop {
                    if let Ok(n) = socket.read_buf(&mut limit).await {
                        if n == 0{
                        println!("Connection {:?} terminated", connection);
                        break;
                            }
                    } else {
                        println!("Connection {:?} terminated", connection);
                        break;
                    }

                }
                    
                }
                println!("Received packet data {:?}", &data[..]);
            });
        }
    }
    async fn talk<T: ToSocketAddrs>(target: T) {
        let mut server = TcpStream::connect(target)
            .await
            .expect("Could not connect to target");
        let mut count: u32 = 1;
        loop {
            server.write_u32(count).await.expect("Could not write data");
            println!("Sent {:?}", count);
            count += 1;
        }
    }

    pub fn client<T: ToSocketAddrs>(target: T) -> QServer {
        let rt = runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(Self::talk(target));

        QServer { rt }
    }
}
