/// The Provider pattern
/// The framework will provide complete abstraction and zero dependence by using a provider pattern
/// Essentially each object that has a dependency will take type provider traits instead of concrete objects
/// We an object needs a particular dependency it will simply call the provider to give it the data
/// How the data is aquired is completley opaque to the requester

pub trait SettingsStore<'a, B>{
    fn add_to_builder(&'a self, builder: B) -> B;
}
pub mod init;
pub mod command;
pub mod memory;
pub mod sync;
pub mod image;
pub mod queue;
pub mod descriptor;
pub mod shader;
pub mod pipelines;
