pub mod bindle;

// Currently, the default way of publishing an application to the Fermyon
// platform is using the Platform's Bindle server.
pub use self::bindle::publish;
