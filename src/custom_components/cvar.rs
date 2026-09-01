use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct Cvar {
    pub name: String,
    pub value: String,
    pub flags: Vec<CvarFlag>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum CvarFlag {
    // --- Core Flags ---
    Server,     // "sv" - Server-side only
    Client,     // "cl" - Client-side only
    Release,    // "release" - Final game builds
    Cheat,      // "cheat" - Requires sv_cheats 1
    Archive,    // "a" - Saved to config.cfg
    Notify,     // "nf" - Notifies players on change
    Replicated, // "rep" - Server forces value onto clients

    // --- Security & Privacy ---
    Protected,       // "prot" - Passwords/Sensitive data (hidden in logs)
    User,            // "user" - Local user setting
    PerUser,         // "per_user" - Unique per Steam account/Profile
    ServerCantQuery, // "server_cant_query" - Hidden from server browser queries

    // --- Execution Rules ---
    ServerCanExecute, // "server_can_execute" - Server can force client to run this
    ClientCanExecute, // "clientcmd_can_execute" - Client can run this on server
    NoRecord,         // "norecord" - Not recorded in demo files

    // --- Plugin & Tooling (Crucial for RCON) ---
    Linked,  // "linked" - External plugins (CS Sharp, MatchZy, etc.)
    Special, // "sp" - Metamod or internal special variables

    // --- Developer / UI Tooling (The "Junk" to filter) ---
    MenuBarItem,      // "menubar_item" - GUI entry in dev tools
    VConsoleFuzzy,    // "vconsole_fuzzy" - Search logic for VConsole
    VConsoleSetFocus, // "vconsole_set_focus" - Window logic for VConsole

    // --- Rare / Internal ---
    DevelopmentOnly, // Usually hidden, used for internal Valve builds
    Hidden,          // Hidden from the standard console
    Defensive,       // Protection against malicious buffer overflows
    Demo,            // Added this
}

pub struct CvarDatabase {
    cvars: HashMap<String, Cvar>,
}

impl CvarDatabase {
    pub fn new(cvar_list: &str) -> Self {
        let mut db = Self {
            cvars: HashMap::new(),
        };

        db.parse(cvar_list);
        db
    }

    fn parse(&mut self, cvar_list: &str) {
        let lines: Vec<&str> = cvar_list.lines().collect();
        if lines.len() <= 4 {
            return;
        }
        for line in &lines[2..lines.len() - 2] {
            self.parse_line(line);
        }
    }

    fn parse_line(&mut self, line: &str) {
        let mut split = line.splitn(4, ':');
        let name = split.next().unwrap().trim();
        let value = split.next().unwrap().trim();
        let flagsss = split.next().unwrap().trim();
        let description = split.next().unwrap().trim();

        let flagss: Vec<&str> = flagsss.split(",").collect();
        let mut flags: Vec<CvarFlag> = vec![];
        for flag in flagss {
            let flag = flag.trim().trim_matches('"');
            match flag.trim() {
                // Identity & Core
                "sv" => flags.push(CvarFlag::Server),
                "cl" => flags.push(CvarFlag::Client),
                "release" => flags.push(CvarFlag::Release),

                // Functional & Replicated
                "a" => flags.push(CvarFlag::Archive),
                "nf" => flags.push(CvarFlag::Notify),
                "rep" => flags.push(CvarFlag::Replicated),
                "cheat" => flags.push(CvarFlag::Cheat),
                "demo" => flags.push(CvarFlag::Demo), // Matches dsp_ convars in your CSV

                // Security & User
                "prot" => flags.push(CvarFlag::Protected),
                "user" => flags.push(CvarFlag::User),
                "per_user" => flags.push(CvarFlag::PerUser),
                "norecord" => flags.push(CvarFlag::NoRecord),

                // Permissions
                "server_can_execute" => flags.push(CvarFlag::ServerCanExecute),
                "server_cant_query" => flags.push(CvarFlag::ServerCantQuery),
                "clientcmd_can_execute" => flags.push(CvarFlag::ClientCanExecute),

                // External Plugins
                "linked" => flags.push(CvarFlag::Linked),
                "sp" => flags.push(CvarFlag::Special),

                // Developer Tooling / UI Junk
                "menubar_item" => flags.push(CvarFlag::MenuBarItem),
                "vconsole_fuzzy" => flags.push(CvarFlag::VConsoleFuzzy),
                "vconsole_set_focus" => flags.push(CvarFlag::VConsoleSetFocus),
                "developmentonly" => flags.push(CvarFlag::DevelopmentOnly),

                // Rare / Internal
                "hidden" => flags.push(CvarFlag::Hidden),
                "defensive" => flags.push(CvarFlag::Defensive),

                _ => (),
            }
        }

        self.cvars.insert(
            name.to_string(),
            Cvar {
                name: name.to_string(),
                value: value.to_string(),
                flags: flags,
                description: description.to_string(),
            },
        );
    }

    pub fn update(&mut self, key: &str, val: &str) {
        self.cvars
            .entry(key.to_string())
            .and_modify(|cvar| cvar.value = val.to_string());
    }

    pub fn get_suggestions(&self, input: &str, user_filters: &HashSet<CvarFlag>) -> Vec<Cvar> {
        let input_lower = input.to_ascii_lowercase();

        if input_lower.is_empty() {
            return Vec::new();
        }

        let mut matches: Vec<(usize, Cvar)> = self
            .cvars
            .values()
            .filter(|cvar| {
                // ---------------------------------------------------------
                // RCON validity
                // ---------------------------------------------------------
                if !self.is_rcon_valid(cvar) {
                    return false;
                }

                // ---------------------------------------------------------
                // User-selected filters
                // ---------------------------------------------------------
                if cvar.flags.iter().any(|flag| user_filters.contains(flag)) {
                    return false;
                }

                true
            })
            .filter_map(|cvar| {
                let name = cvar.name.to_ascii_lowercase();

                let score = if name == input_lower {
                    // -----------------------------------------------------
                    // Exact match
                    // -----------------------------------------------------
                    0
                } else if name.starts_with(&input_lower) {
                    // -----------------------------------------------------
                    // Starts with
                    //
                    // Example:
                    // "mp_pa" -> "mp_pause"
                    // -----------------------------------------------------
                    name.len()
                } else if let Some(position) = name.find(&input_lower) {
                    // -----------------------------------------------------
                    // Contains
                    //
                    // Example:
                    // "pause" -> "mp_pause"
                    // -----------------------------------------------------
                    100 + position * 2 + name.len()
                } else {
                    // -----------------------------------------------------
                    // Fuzzy subsequence
                    //
                    // Every character of the query must occur in order,
                    // but they don't have to be adjacent.
                    // -----------------------------------------------------
                    let name_chars: Vec<char> = name.chars().collect();
                    let query_chars: Vec<char> = input_lower.chars().collect();

                    let mut query_index = 0;
                    let mut score = 200 + name_chars.len();
                    let mut last_match: Option<usize> = None;

                    for (index, ch) in name_chars.iter().enumerate() {
                        if query_index >= query_chars.len() {
                            break;
                        }

                        if *ch != query_chars[query_index] {
                            continue;
                        }

                        // Penalize gaps between matched characters.
                        if let Some(previous) = last_match {
                            score += index.saturating_sub(previous + 1);
                        }

                        // Bonus when a match starts after '_'.
                        if index > 0 && name_chars[index - 1] == '_' {
                            score = score.saturating_sub(10);
                        }

                        // Bonus for matching at the beginning.
                        if index == 0 {
                            score = score.saturating_sub(10);
                        }

                        last_match = Some(index);
                        query_index += 1;
                    }

                    // The complete query wasn't matched.
                    if query_index != query_chars.len() {
                        return None;
                    }

                    score
                };

                Some((
                    score,
                    Cvar {
                        name: cvar.name.clone(),
                        value: cvar.value.clone(),
                        flags: cvar.flags.clone(),
                        description: cvar.description.clone(),
                    },
                ))
            })
            .collect();

        // Lower score = better match.
        matches.sort_by(|a, b| a.0.cmp(&b.0));

        // Only keep the best 20 matches.
        matches.truncate(20);

        // The popup is above the input field, so put the best match
        // at the bottom, directly above the input.
        matches.reverse();

        matches.into_iter().map(|(_, cvar)| cvar).collect()
    }

    pub fn is_rcon_valid(&self, cvar: &Cvar) -> bool {
        let f = &cvar.flags;

        // 1. HIDDEN & UI JUNK
        // MenuBarItem is purely for the developer GUI.
        // Hidden cvars are usually internal engine states.
        if f.contains(&CvarFlag::MenuBarItem) || f.contains(&CvarFlag::Hidden) {
            return false;
        }

        // 2. LOCAL-ONLY LOGIC
        // These only affect the player's local machine or demo playback.
        if f.contains(&CvarFlag::Demo)
            || f.contains(&CvarFlag::PerUser)
            || f.contains(&CvarFlag::User)
        {
            return false;
        }

        // 3. THE "PURE CLIENT" RULE
        // If it is flagged 'cl' but NOT 'sv', it's a client setting (e.g., cl_drawhud, volume).
        // RCON cannot change these for players.
        let is_pure_client = f.contains(&CvarFlag::Client) && !f.contains(&CvarFlag::Server);
        if is_pure_client {
            return false;
        }

        // 4. THE INCLUSION RULE
        // - Server (sv)
        // - Plugins (Linked/Special)
        // - Engine Commands (Usually just 'Release' - like 'map', 'kick', 'changelevel')
        true
    }
}
