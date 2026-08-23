use regex::Regex;

#[derive(Debug, Clone)]
pub struct ServerCvars {
    pub values: Vec<(String, String)>,
}

impl ServerCvars {
    pub fn parse(input: &str) -> Option<Self> {
        let mut values = Vec::new();

        let ts_re =
            Regex::new(r#"(?m)^(?:\[LOG\]\s+)?\d{2}/\d{2}/\d{4} - \d{2}:\d{2}:\d{2}\.\d{3} - "#)
                .ok()?;

        let cleaned = ts_re.replace_all(input, "").trim().to_string();

        let cvar_re = Regex::new(r#"^"(?P<name>[^"]+)" = "(?P<value>[^"]*)"$"#).ok()?;

        let mut in_block = false;

        for line in cleaned.lines() {
            let line = line.trim();

            if line.trim() == "server cvars start" {
                in_block = true;
                continue;
            }

            if line.trim() == "server cvars end" {
                if !in_block {
                    return None;
                }

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
