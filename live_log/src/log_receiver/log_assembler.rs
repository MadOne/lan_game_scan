const MAX_JSON_LINES: usize = 80;
const MAX_CVAR_LINES: usize = 1028;

pub struct LogAssembler {
    json_buffer: Option<Vec<String>>,
    cvar_buffer: Option<Vec<String>>,
}

impl LogAssembler {
    pub fn new() -> Self {
        Self {
            json_buffer: None,
            cvar_buffer: None,
        }
    }

    pub fn process_line(&mut self, line: &str) -> Vec<String> {
        let line = line.trim();

        if line.is_empty() {
            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // We are already collecting JSON
        // ---------------------------------------------------------------------

        if let Some(buffer) = self.json_buffer.as_mut() {
            buffer.push(line.to_string());

            if line.contains("JSON_END") {
                let buffer = self.json_buffer.take();

                return match buffer {
                    Some(buffer) => vec![buffer.join("\n")],
                    None => Vec::new(),
                };
            }

            if buffer.len() >= MAX_JSON_LINES {
                let line_count = buffer.len();

                self.json_buffer = None;

                log::warn!(
                    target: "live_log",
                    "JSON buffer exceeded {} lines; discarding incomplete block ({} lines)",
                    MAX_JSON_LINES,
                    line_count
                );

                return Vec::new();
            }

            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // We are already collecting a CVar dump
        // ---------------------------------------------------------------------

        if let Some(buffer) = self.cvar_buffer.as_mut() {
            buffer.push(line.to_string());

            if line.to_ascii_lowercase().contains("server cvars end") {
                let buffer = self.cvar_buffer.take();

                return match buffer {
                    Some(buffer) => vec![buffer.join("\n")],
                    None => Vec::new(),
                };
            }

            if buffer.len() >= MAX_CVAR_LINES {
                let line_count = buffer.len();

                self.cvar_buffer = None;

                log::warn!(
                    target: "live_log",
                    "CVar buffer exceeded {} lines; discarding incomplete block ({} lines)",
                    MAX_CVAR_LINES,
                    line_count
                );

                return Vec::new();
            }

            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // Start of a new JSON block
        // ---------------------------------------------------------------------

        if line.contains("JSON_BEGIN") {
            let mut buffer = Vec::with_capacity(MAX_JSON_LINES);
            buffer.push(line.to_string());

            self.json_buffer = Some(buffer);

            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // Start of a new CVar dump
        // ---------------------------------------------------------------------

        if line.to_ascii_lowercase().contains("server cvars start") {
            let mut buffer = Vec::with_capacity(MAX_CVAR_LINES);
            buffer.push(line.to_string());

            self.cvar_buffer = Some(buffer);

            return Vec::new();
        }

        // ---------------------------------------------------------------------
        // Normal log line
        // ---------------------------------------------------------------------

        vec![line.to_string()]
    }
}
