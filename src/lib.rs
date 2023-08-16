#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)] // the code is generated
#![allow(improper_ctypes)]            // ignore warning not FFI-safe 

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
