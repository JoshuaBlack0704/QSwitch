use qserver::QServer;
 fn main(){
    println!("Hello world");
    let client = QServer::client("127.0.0.1:8080");
 }