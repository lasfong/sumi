#![warn(clippy::all, rust_2018_idioms)]

pub mod alerts;
pub mod backtest;
pub mod data;
pub mod drawings;
pub mod indicators;
pub mod journal;
pub mod model;
pub mod replay;
pub mod scanner;
pub mod trading;
pub mod viewport;

/// Semantic v2.0 alias exposing the core domain models (`Candle`, `PriceBounds`, ...).
pub mod domain {
    pub use crate::model::*;
}
