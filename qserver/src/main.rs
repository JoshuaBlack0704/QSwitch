use qserver::QServer;

fn main() {
    let mut server = QServer::new("127.0.0.1:8080");
    server.shutdown();
}
