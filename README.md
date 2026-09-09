# LanGameScan

**LanGameScan** is a LAN server browser and administration tool written in Rust and Dioxus.

The main focus is **Counter-Strike 2**, but the long-term goal is broader LAN game discovery and server administration.

## Features

- LAN server discovery
- Server list with favorites and server details
- Server information and player lists
- RCON support for:
  - Source / Source 2
  - GoldSrc
  - Quake 3
- RCON console with command history and CVar suggestions
- CS2 live server logs
- Parsed game events and match state
- MatchZy integration for CS2
- Application-wide log viewer with:
  - log level filters
  - crate filters
  - target filtering
  - text search
  - timestamps
  - compact targets
- Persistent server and RCON settings
- Desktop, Windows, Linux and Android builds

## Project Structure

The project is split into several Rust crates:

- `lan_game_scan` — application and user interface
- `lan_scan` — LAN discovery and server query protocols
- `cbz_rcon` — RCON clients and protocol implementations
- `live_log` — HTTP live-log receiver and game log parsing

## Development

Install the [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/) and start the application with:

```bash
dx serve