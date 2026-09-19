// -----------------------------------------------------------------------------
// parser.rs
// -----------------------------------------------------------------------------

use crate::{
    _parser::{
        cs16::{self, cs16_build_patterns, TS_BLOCK as CS16_TS_BLOCK},
        cs2::{self, cs2_build_patterns, TS_BLOCK as CS2_CS_BLOCK},
        css::{self, css_build_patterns, TS_BLOCK as CSS_TS_BLOCK},
        patterns::LogPattern,
        types::{LogEvent, LogType, ParsedLine},
    },
    game::Game,
};
use regex::{Captures, Regex, RegexSet};

pub struct LogParser {
    re_ts: Regex,
    pattern_set: RegexSet,
    patterns: Vec<LogPattern>,
}

impl LogParser {
    pub fn new(game: Game) -> Self {
        let patterns = match game {
            Game::Cs2 => cs2::cs2_build_patterns(),
            Game::Css => css::css_build_patterns(),
            Game::Cs16 => cs16::cs16_build_patterns(),
        };
        let ts_block = match game {
            Game::Cs2 => CS2_CS_BLOCK,
            Game::Css => CSS_TS_BLOCK,
            Game::Cs16 => CS16_TS_BLOCK,
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

    fn normalize_timestamp(&self, line: &str) -> String {
        let line = line.strip_prefix("L ").unwrap_or(line);

        // GoldSrc:
        // 09/17/2026 - 12:23:53: message
        //
        // Normalize to:
        // 09/17/2026 - 12:23:53.000 - message
        if line.len() >= 23 && line.as_bytes()[11] == b'-' && line.as_bytes()[21] == b':' {
            let mut normalized = String::with_capacity(line.len() + 6);
            normalized.push_str(&line[..21]);
            normalized.push_str(".000 - ");
            normalized.push_str(&line[23..]);
            return normalized;
        }

        line.to_string()
    }

    pub fn normalize_steamids(&self, line: &str) -> String {
        let re = Regex::new(r"\[BOT\]|\bBOT\b|\[U:\d+:(\d+)\]|STEAM_\d+:(\d+):(\d+)")
            .expect("valid Steam ID regex");

        re.replace_all(line, |caps: &Captures| {
            let value = caps.get(0).map(|m| m.as_str()).unwrap_or_default();

            if value == "BOT" || value == "[BOT]" {
                return "BOT".to_string();
            }

            if let Some(account_id) = caps.get(1) {
                return format!("[U:1:{}]", account_id.as_str());
            }

            if let (Some(y), Some(z)) = (caps.get(2), caps.get(3)) {
                let y = y.as_str().parse::<u64>();
                let z = z.as_str().parse::<u64>();

                if let (Ok(y), Ok(z)) = (y, z) {
                    return format!("[U:1:{}]", z * 2 + y);
                }
            }

            value.to_string()
        })
        .into_owned()
    }
}
