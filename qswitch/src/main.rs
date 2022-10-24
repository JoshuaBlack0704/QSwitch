use qserver::UdpServiceListener;
use tokio::{net::UdpSocket, runtime};
 fn main(){
    println!("Hello world");
    let rt = runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let s1 = UdpServiceListener::start("127.0.0.1:0", &rt);
    let s2 = UdpServiceListener::start("127.0.0.1:0", &rt);
    let s3 = UdpServiceListener::start("127.0.0.1:0", &rt);
    s2.initiate_udp_link(s1.get_local_addr().unwrap());
    s3.initiate_udp_link(s1.get_local_addr().unwrap());
    let l2 = s2.get_new_link().unwrap();
    let l3 = s3.get_new_link().unwrap();
    l2.tx().send((5,[1;500])).unwrap();
    l3.tx().send((5,[1;500])).unwrap();
    s1.stop(&rt);
    loop{}
 }