#![warn(clippy::all, rust_2018_idioms)]

pub mod app;
pub mod chart;
pub mod data;
pub mod indicators;
pub mod model;
pub mod viewport;

pub use app::SumiWorkstationApp;
