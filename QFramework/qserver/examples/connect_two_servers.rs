use qserver::LocalServer;

fn main(){
    let s1 = LocalServer::new(None, true, None);
    let s2 = LocalServer::new(None, true, Some(s1.get_runtime()));
    LocalServer::connect_to_server(s1.clone(), s2.local_address());
    s1.idle_async();
}