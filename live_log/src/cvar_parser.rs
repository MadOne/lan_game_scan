use regex::Regex;

use crate::_parser::sourceengines::{GOLDSRC_TS_BLOCK, SOURCE2_TS_BLOCK};

#[derive(Debug, Clone)]
pub struct ServerCvars {
    pub values: Vec<(String, String)>,
}

impl ServerCvars {
    pub fn parse(input: &str) -> Option<Self> {
        println!("---------Parser started!---------------");
        let source2_ts_re = Regex::new(&format!(r"(?m)^{}", SOURCE2_TS_BLOCK)).ok()?;
        let goldsrc_ts_re = Regex::new(&format!(r"(?m)^{}", GOLDSRC_TS_BLOCK)).ok()?;

        let cleaned = source2_ts_re.replace_all(input, "");
        let cleaned = goldsrc_ts_re.replace_all(&cleaned, "");

        let cleaned = cleaned
            .lines()
            .map(|line| line.strip_prefix("Server cvar ").unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n");

        println!("CLEANED!");
        println!("{}", &cleaned);
        println!("END CLEANED!");
        let cvar_re = Regex::new(r#"^"(?P<name>[^"]+)" = "(?P<value>[^"]*)"$"#).ok()?;

        let mut values = Vec::new();
        let mut in_block = false;

        for line in cleaned.lines() {
            let line = line.trim();

            if line.to_ascii_lowercase() == "server cvars start" {
                in_block = true;
                continue;
            }

            if line.to_ascii_lowercase() == "server cvars end" {
                if !in_block {
                    return None;
                }
                println!("returning parsed cvarlist");
                return Some(Self { values });
            }

            if !in_block {
                continue;
            }

            if let Some(caps) = cvar_re.captures(line) {
                let name = caps.name("name")?.as_str().to_string();
                let value = caps.name("value")?.as_str().to_string();

                values.push((name, value));
            }
        }

        None
    }
}
