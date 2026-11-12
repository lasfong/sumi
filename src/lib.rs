#![warn(clippy::all, rust_2018_idioms)]

pub mod alerts;
pub mod app;
pub mod chart;
pub mod data;
pub mod drawings;
pub mod indicators;
pub mod model;
pub mod replay;
pub mod scanner;
pub mod trading;
pub mod viewport;

pub use app::SumiWorkstationApp;
