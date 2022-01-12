#![allow(
    clippy::needless_question_mark,
    clippy::too_many_arguments,
    clippy::map_flatten,
    clippy::type_complexity,
    clippy::module_inception
)]
#[macro_use]
extern crate log;

pub mod api;
pub mod engine;
pub mod input_system;
pub mod logger;
pub mod physics;
pub mod renderer;
pub mod time;
