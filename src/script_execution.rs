use std::ffi::OsString;

use futures::channel::oneshot::Sender;
use image::{ImageBuffer, Rgba};

use crate::script_option::ScriptOption;

pub struct Executor {
    pub script_option: ScriptOption, 
    pub image1: OsString, 
    pub image2: Option<OsString>, 
    pub other_args: String,
    pub return_channel: Sender<ImageBuffer<Rgba<u8>, Vec<u8>>>, 
}