// #![feature(fn_traits)]
#![feature(new_uninit)]
// #![feature(unboxed_closures)]

// API
mod sender;
pub use self::sender::*;

mod sink;
pub use self::sink::*;

mod source;
pub use self::source::*;

mod receiver;
pub use self::receiver::*;

// Internal
