// -----------------------------------------------------------------------------
// parser.rs
// -----------------------------------------------------------------------------

use crate::{
    _parser::{
        cs16::{cs16_build_patterns, CS16_TS_BLOCK},
        cs2::{cs2_build_patterns, CS2_TS_BLOCK},
        css::{css_build_patterns, CSS_TS_BLOCK},
        dods::{dods_build_patterns, DODS_TS_BLOCK},
        no_parsing::{no_build_patterns, NO_TS_BLOCK},
        patterns::LogPattern,
        sourceengines::{GOLDSRC_TS_BLOCK, SOURCE2_TS_BLOCK, SOURCE_TS_BLOCK},
        tf2::{tf2_build_patterns, TF2_TS_BLOCK},
        types::{LogEvent, LogType, ParsedLine},
    },
    game::Game,
};
use regex::{Regex, RegexSet};

pub struct LogParser {
    re_ts: Regex,
    pattern_set: RegexSet,
    patterns: Vec<LogPattern>,
}

impl LogParser {
    pub fn new(game: Game) -> Self {
        let patterns = match game {
            Game::Cs2 => cs2_build_patterns(),
            Game::Css => css_build_patterns(),
            Game::Cs16 => cs16_build_patterns(),
            Game::DoDS => dods_build_patterns(),
            Game::GenericGoldSrc => cs16_build_patterns(),
            Game::GenericSource => css_build_patterns(),
            Game::GenericSource2 => cs2_build_patterns(),
            Game::TF2 => tf2_build_patterns(),
            Game::CoD4 => no_build_patterns(),
            Game::UT2k4 => no_build_patterns(),
            Game::GenericQuake3 => no_build_patterns(),
            Game::GenericGameSpy => no_build_patterns(),
        };
        let ts_block = match game {
            Game::Cs2 => CS2_TS_BLOCK,
            Game::Css => CSS_TS_BLOCK,
            Game::Cs16 => CS16_TS_BLOCK,
            Game::DoDS => DODS_TS_BLOCK,
            Game::GenericGoldSrc => GOLDSRC_TS_BLOCK,
            Game::GenericSource => SOURCE_TS_BLOCK,
            Game::GenericSource2 => SOURCE2_TS_BLOCK,
            Game::TF2 => TF2_TS_BLOCK,
            Game::CoD4 => NO_TS_BLOCK,
            Game::UT2k4 => NO_TS_BLOCK,
            Game::GenericQuake3 => NO_TS_BLOCK,
            Game::GenericGameSpy => NO_TS_BLOCK,
        };

        //let patterns = cs2_build_patterns();

        let pattern_set = RegexSet::new(patterns.iter().map(|pattern| pattern.regex.as_str()))
            .expect("Failed to compile parser regex patterns");

        Self {
            re_ts: Regex::new(&format!(r"(?s)^{}(?P<content>.*)$", ts_block))
                .expect("Failed to compile timestamp regex"),

            pattern_set,
            patterns,
        }
    }

    // -------------------------------------------------------------------------
    // PARSE
    // -------------------------------------------------------------------------

    pub fn parse(&self, line: &str) -> ParsedLine {
        let mut timestamp = String::new();
        let mut content = line;

        let line = line.strip_prefix("L ").unwrap_or(line);

        // -------------------------------------------------------------
        // Timestamp / content extraction
        // -------------------------------------------------------------

        if let Some(caps) = self.re_ts.captures(line) {
            if let Some(ts) = caps.name("ts") {
                timestamp = ts.as_str().to_string();
            }

            if let Some(cnt) = caps.name("content") {
                content = cnt.as_str();
            }
        }

        // -------------------------------------------------------------
        // Pattern matching
        // -------------------------------------------------------------

        let matches: Vec<usize> = self.pattern_set.matches(content).into_iter().collect();

        let event = match matches.as_slice() {
            // ---------------------------------------------------------
            // No pattern matched
            // ---------------------------------------------------------
            [] => LogEvent::Unknown,

            // ---------------------------------------------------------
            // Exactly one pattern matched
            // ---------------------------------------------------------
            [index] => {
                let pattern = &self.patterns[*index];

                let caps = match pattern.regex.captures(content) {
                    Some(caps) => caps,

                    None => {
                        log::warn!(
                            target: "live_log::parser",
                            "RegexSet mismatch for pattern '{}': {}",
                            pattern.id,
                            content
                        );

                        return ParsedLine {
                            raw: line.to_string(),
                            timestamp,
                            event: LogEvent::Unknown,
                            log_type: LogType::Unknown,
                            pretty: String::new(),
                        };
                    }
                };

                match pattern.parse(content, &caps) {
                    Some(event) => event,

                    None => {
                        log::warn!(
                            target: "live_log::parser",
                            "Pattern '{}' matched but failed to parse: {}",
                            pattern.id,
                            content
                        );

                        LogEvent::Unknown
                    }
                }
            }

            // ---------------------------------------------------------
            // Multiple patterns matched
            // ---------------------------------------------------------
            multiple => {
                log::warn!(
                    target: "live_log::parser",
                    "AMBIGUOUS LOG LINE: {} matches: {:?}\n{}",
                    multiple.len(),
                    multiple,
                    content
                );

                LogEvent::Unknown
            }
        };

        // -------------------------------------------------------------
        // Derive LogType from the actual LogEvent.
        // -------------------------------------------------------------

        let log_type = event.kind();

        // -------------------------------------------------------------
        // Pretty output
        //
        // Use the matched pattern when possible.
        // For UNKNOWN / ambiguous events there is no pattern to format.
        // -------------------------------------------------------------

        let pretty = match matches.as_slice() {
            [index] => {
                let pattern = &self.patterns[*index];

                pattern.pretty(&event)
            }

            _ => String::new(),
        };

        ParsedLine {
            raw: line.to_string(),
            timestamp,
            event,
            log_type,
            pretty,
        }
    }
}
