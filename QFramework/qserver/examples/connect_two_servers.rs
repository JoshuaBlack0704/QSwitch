use qserver::LocalServer;

fn main(){
    let s1 = LocalServer::new(None, true, None, None);
    let s2 = LocalServer::new(None, true, Some(s1.get_runtime()), Some(s1.local_address()));
    s1.idle_async();
}