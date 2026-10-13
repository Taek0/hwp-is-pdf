#![allow(dead_code)]
pub mod ole_chart {
    pub mod grid;
    pub mod orientation;
    pub mod parser;
    pub mod legacy_presentation;
}
pub use ole_chart::parser::{parse_ole_chart_contents,OleChart};
