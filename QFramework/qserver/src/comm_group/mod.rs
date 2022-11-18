use crate::CommGroup;


pub(crate) enum CommGroupHeader<Key: Clone>{
    ID(Key)
}

impl<Key: Clone> CommGroup<Key>{
    pub fn new(key: Key) -> CommGroup<Key> {
        CommGroup{ key }
    }
    
}
