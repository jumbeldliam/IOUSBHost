//FIXME: should not have to allow all warnings, should change something for bindgen
#![allow(warnings)]
use objc::msg_send;
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
