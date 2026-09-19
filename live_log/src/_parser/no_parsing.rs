use crate::_parser::patterns::{LogPattern, LogPatternBlocks};

pub const NO_TS_BLOCK: &str = "";
pub const STEAMID3_BLOCK: &str = "";

pub fn no_build_patterns() -> Vec<LogPattern> {
    let _blocks = LogPatternBlocks::new(NO_TS_BLOCK, STEAMID3_BLOCK);
    vec![]
}
