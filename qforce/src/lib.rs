mod communication {
    pub trait Interconnect {}
    pub trait InterconnectOrigin<MessageType, T: Interconnect> {
        fn get_interconnect(&self) -> T;
        fn broadcast(&self, message: MessageType);
    }

    pub struct ChannelHub<MessageType> {
        channel: (
            flume::Sender<crossbeam::channel::Sender<MessageType>>,
            flume::Receiver<crossbeam::channel::Receiver<MessageType>>,
        ),
    }
}
