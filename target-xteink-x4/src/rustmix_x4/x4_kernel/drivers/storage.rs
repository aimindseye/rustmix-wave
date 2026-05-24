// sd card file operations
//
// all I/O through embedded-sdmmc AsyncVolumeManager; functions are
// synchronous, wrapping async ops with poll_once (SPI bus is blocking
// so every .await resolves immediately)
//
// returns the unified Error type (re-exported as StorageError for
// backward compat); apps receive it through KernelHandle

use core::ops::ControlFlow;

use embedded_sdmmc::{Mode, RawDirectory};

use crate::rustmix_x4::x4_kernel::drivers::sdcard::{SdStorage, SdStorageInner, poll_once};
use crate::rustmix_x4::x4_kernel::error::{Error, ErrorKind};

pub const RUSTMIX_DIR: &str = "RUSTMIX";
pub const LEGACY_X4_DIR: &str = "_x4";
// Backward-compatible name used by older app/cache helpers. New Rustmix
// firmware data lives under /RUSTMIX, not /_x4.
pub const X4_DIR: &str = RUSTMIX_DIR;
pub const TITLES_FILE: &str = "TITLES.BIN";
pub const TITLE_CAP: usize = 64;
pub const MAX_NESTED_STORAGE_PATH_COMPONENTS: usize = 8;

const DEFAULT_SETTINGS_TXT: &[u8] = b"# rustmix-os settings\n# Created automatically on first boot. Edit from Settings or Wi-Fi setup.\n\n# power settings\nsleep_timeout=10\nghost_clear=10\n\n# font settings\nbook_font=2\nui_font=2\n\n# reading settings (0=Compact, 1=Default, 2=Relaxed, 3=Spacious)\nreading_theme=1\n\n# reader preferences\nshow_progress=1\nbionic_reading=0\nguide_dots=0\nsunlight_fading_fix=0\nreader_orientation=0\nprepared_font_profile=1\nprepared_fallback_policy=0\nreader_font_source=0\nreader_sd_font_slot=0\nreader_sd_font_id=\n\n# display preferences (persisted only)\nui_font_source=1\ndisplay_refresh_mode=1\ndisplay_invert_colors=0\ndisplay_contrast_high=0\n\n# control settings\nswap_buttons=0\n\n# wifi credentials for upload mode\nwifi_ssid=\nwifi_pass=\nwifi_default=0\n\n# saved Wi-Fi profiles stored in SETTINGS.TXT\nwifi_profile_0_name=Home\nwifi_profile_0_ssid=\nwifi_profile_0_pass=\nwifi_profile_1_name=Work\nwifi_profile_1_ssid=\nwifi_profile_1_pass=\nwifi_profile_2_name=Other\nwifi_profile_2_ssid=\nwifi_profile_2_pass=\n";

const DEFAULT_TIME_TXT: &[u8] = b"timezone=America/New_York\nlast_sync_unix=\nlast_sync_monotonic_ms=\nlast_sync_ok=0\nlast_sync_source=\nlast_sync_error=\nlast_sync_ip=\ndisplay_offset_minutes=-300\n";

const DEFAULT_PROVISION_TXT: &[u8] = b"rustmix_sd_provision=1\npolicy=seed-missing-files-only\n";

const DEFAULT_SLEEP_FILES: &[(&str, &[u8])] = &[
    (
        "SLEEP06.BMP",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/SLEEP/SLEEP06.BMP"
        )),
    ),
    (
        "SLEEP05.BMP",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/SLEEP/SLEEP05.BMP"
        )),
    ),
    (
        "SLEEP04.BMP",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/SLEEP/SLEEP04.BMP"
        )),
    ),
    (
        "SLEEP03.BMP",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/SLEEP/SLEEP03.BMP"
        )),
    ),
    (
        "SLEEP02.BMP",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/SLEEP/SLEEP02.BMP"
        )),
    ),
    (
        "SLEEP01.BMP",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/SLEEP/SLEEP01.BMP"
        )),
    ),
    (
        "SLEEP.BMP",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/SLEEP/SLEEP.BMP"
        )),
    ),
    (
        "SLEEP00.BMP",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/SLEEP/SLEEP00.BMP"
        )),
    ),
    (
        "README.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/SLEEP/README.TXT"
        )),
    ),
];

const DEFAULT_DICTIONARY_INDEX_TXT: &[u8] = b"A|DATA/A.JSN\nB|DATA/B.JSN\nC|DATA/C.JSN\nD|DATA/D.JSN\nE|DATA/E.JSN\nF|DATA/F.JSN\nG|DATA/G.JSN\nH|DATA/H.JSN\nI|DATA/I.JSN\nJ|DATA/J.JSN\nK|DATA/K.JSN\nL|DATA/L.JSN\nM|DATA/M.JSN\nN|DATA/N.JSN\nO|DATA/O.JSN\nP|DATA/P.JSN\nQ|DATA/Q.JSN\nR|DATA/RUST.JSN\nS|DATA/S.JSN\nT|DATA/T.JSN\nU|DATA/U.JSN\nV|DATA/V.JSN\nW|DATA/W.JSN\nX|DATA/XTEINK.JSN\nY|DATA/Y.JSN\nZ|DATA/Z.JSN\n";
const DEFAULT_DICTIONARY_FALLBACK_JSN: &[u8] = br#"{"RUST":[{"def":"A systems programming language used by Rustmix firmware.","pos":"noun"}],"RUSTMIX":[{"def":"Reference Rust firmware for the Xteink X4.","pos":"noun"}],"XTEINK":[{"def":"The Xteink X4 e-ink reader hardware target.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_A_JSN: &[u8] = br#"{"APPLE":[{"def":"A common round fruit.","pos":"noun"}],"APP":[{"def":"A small software application.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_B_JSN: &[u8] = br#"{"BOAT":[{"def":"A small vessel for travel on water.","pos":"noun"}],"BOOK":[{"def":"A written or printed work made of pages.","pos":"noun"}],"BAT":[{"def":"A club used in games, or a flying mammal.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_C_JSN: &[u8] = br#"{"CACHE":[{"def":"Stored data kept for faster access.","pos":"noun"}],"CARD":[{"def":"A small rectangular piece of paper or data item.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_D_JSN: &[u8] = br#"{"DEVICE":[{"def":"A piece of electronic hardware.","pos":"noun"}],"DATA":[{"def":"Facts or information used by a program.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_E_JSN: &[u8] = br#"{"EINK":[{"def":"A low power electronic paper display technology.","pos":"noun"}],"ENTRY":[{"def":"One item in a list or dictionary.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_F_JSN: &[u8] = br#"{"FIRMWARE":[{"def":"Software stored on a device to control hardware.","pos":"noun"}],"FONT":[{"def":"A set of glyphs used to display text.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_G_JSN: &[u8] = br#"{"GAME":[{"def":"An activity played for challenge or fun.","pos":"noun"}],"GLYPH":[{"def":"A visual shape representing a character.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_H_JSN: &[u8] = br#"{"HOME":[{"def":"The main starting screen.","pos":"noun"}],"HARDWARE":[{"def":"The physical parts of a computer or device.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_I_JSN: &[u8] = br#"{"INDEX":[{"def":"A file or list used to find data quickly.","pos":"noun"}],"INPUT":[{"def":"Information or actions sent to a program.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_J_JSN: &[u8] = br#"{"JSON":[{"def":"A text format for structured data.","pos":"noun"}],"JOURNAL":[{"def":"A regular written record.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_K_JSN: &[u8] = br#"{"KEY":[{"def":"A button or identifier used for access or input.","pos":"noun"}],"KERNEL":[{"def":"The core part of a system that manages resources.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_L_JSN: &[u8] = br#"{"LUA":[{"def":"A small scripting language used for apps.","pos":"noun"}],"LIBRARY":[{"def":"A collection of books or reusable software.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_M_JSN: &[u8] = br#"{"MENU":[{"def":"A list of commands or options.","pos":"noun"}],"MEMORY":[{"def":"Storage used by a program while it runs.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_N_JSN: &[u8] = br#"{"NOTE":[{"def":"A short written record.","pos":"noun"}],"NETWORK":[{"def":"Connected devices that exchange data.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_O_JSN: &[u8] = br#"{"OPEN":[{"def":"To make available for use or reading.","pos":"verb"}],"OS":[{"def":"Operating system.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_P_JSN: &[u8] = br#"{"PAGE":[{"def":"One screen or sheet of content.","pos":"noun"}],"PATH":[{"def":"A location of a file or folder.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_Q_JSN: &[u8] = br#"{"QUERY":[{"def":"A search request.","pos":"noun"}],"QUICK":[{"def":"Fast or brief.","pos":"adjective"}]}"#;
const DEFAULT_DICTIONARY_R_JSN: &[u8] = br#"{"RUST":[{"def":"A systems programming language used by Rustmix firmware.","pos":"noun"}],"RUSTMIX":[{"def":"Reference Rust firmware for the Xteink X4.","pos":"noun"}],"READER":[{"def":"An app for opening and reading books.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_S_JSN: &[u8] = br#"{"SD":[{"def":"Secure Digital storage card.","pos":"noun"}],"SHARD":[{"def":"One small part of a larger data set.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_T_JSN: &[u8] = br#"{"TEXT":[{"def":"Written characters displayed or stored as data.","pos":"noun"}],"TOOL":[{"def":"A utility used to perform a task.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_U_JSN: &[u8] = br#"{"UPLOAD":[{"def":"To transfer data to a device or server.","pos":"verb"}],"USER":[{"def":"A person who operates a system.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_V_JSN: &[u8] = br#"{"VIEW":[{"def":"To look at or display.","pos":"verb"}],"VERSION":[{"def":"A particular release of software or data.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_W_JSN: &[u8] = br#"{"WIFI":[{"def":"Wireless networking technology.","pos":"noun"}],"WORD":[{"def":"A unit of language carrying meaning.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_X_JSN: &[u8] = br#"{"XTEINK":[{"def":"The Xteink X4 e-ink reader hardware target.","pos":"noun"}],"XTEINK X4":[{"def":"ESP32-C3 based e-ink reader used by this firmware.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_Y_JSN: &[u8] = br#"{"YES":[{"def":"An affirmative answer.","pos":"noun"}],"YIELD":[{"def":"To produce or give way.","pos":"verb"}]}"#;
const DEFAULT_DICTIONARY_Z_JSN: &[u8] = br#"{"ZIP":[{"def":"A compressed archive file format.","pos":"noun"}],"ZERO":[{"def":"The number 0.","pos":"noun"}]}"#;
const DEFAULT_DICTIONARY_RUST_JSN: &[u8] = DEFAULT_DICTIONARY_R_JSN;
const DEFAULT_DICTIONARY_XTEINK_JSN: &[u8] = DEFAULT_DICTIONARY_X_JSN;

const DEFAULT_FONTS: &[(&str, &[u8])] = &[
    (
        "MANIFEST.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/FONTS/MANIFEST.TXT"
        )),
    ),
    (
        "README.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/FONTS/README.TXT"
        )),
    ),
    (
        "UIFONTS.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/FONTS/UIFONTS.TXT"
        )),
    ),
    (
        "CHARIS18.VFN",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/FONTS/CHARIS18.VFN"
        )),
    ),
    (
        "BITTER18.VFN",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/FONTS/BITTER18.VFN"
        )),
    ),
    (
        "INTER14.VFN",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/FONTS/INTER14.VFN"
        )),
    ),
    (
        "LEXEND18.VFN",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/FONTS/LEXEND18.VFN"
        )),
    ),
    (
        "LEXUI14.VFN",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/FONTS/LEXUI14.VFN"
        )),
    ),
];

const DEFAULT_FREECELL_APP_TOM: &[u8] = b"id = \"freecell\"
name = \"FreeCell\"
category = \"Games\"
type = \"activity\"
version = \"0.1.0\"
entry = \"MAIN.LUA\"
capabilities = [\"display\", \"input\", \"storage\"]
";
const DEFAULT_FREECELL_MAIN_LUA: &[u8] = b"-- FreeCell starter app
display_title = \"FreeCell\"
display_line1 = \"Starter Lua app installed by Rustmix.\"
display_line2 = \"Replace MAIN.LUA over Wi-Fi Transfer to customize.\"
display_footer = \"Folder: /RUSTMIX/APPS/FREECELL\"
";
const DEFAULT_FREECELL_CARDS_TXT: &[u8] = b"A,S
2,S
3,S
4,S
";

const DEFAULT_SOLITAIR_APP_TOM: &[u8] = b"id = \"solitaire\"
name = \"Solitaire\"
category = \"Games\"
type = \"activity\"
version = \"0.1.0\"
entry = \"MAIN.LUA\"
capabilities = [\"display\", \"input\", \"storage\"]
";
const DEFAULT_SOLITAIR_MAIN_LUA: &[u8] = b"-- Solitaire starter app
display_title = \"Solitaire\"
display_line1 = \"Starter Lua app installed by Rustmix.\"
display_line2 = \"Replace MAIN.LUA over Wi-Fi Transfer to customize.\"
display_footer = \"Folder: /RUSTMIX/APPS/SOLITAIR\"
";
const DEFAULT_SOLITAIR_CARDS_TXT: &[u8] = b"stock=24
waste=0
foundation=0
";

const DEFAULT_LUDO_APP_TOM: &[u8] = b"id = \"ludo\"
name = \"Ludo\"
category = \"Games\"
type = \"activity\"
version = \"0.1.0\"
entry = \"MAIN.LUA\"
capabilities = [\"display\", \"input\", \"storage\"]
";
const DEFAULT_LUDO_MAIN_LUA: &[u8] = b"-- Ludo starter app
display_title = \"Ludo\"
display_line1 = \"Starter Lua app installed by Rustmix.\"
display_line2 = \"Replace MAIN.LUA over Wi-Fi Transfer to customize.\"
display_footer = \"Folder: /RUSTMIX/APPS/LUDO\"
";
const DEFAULT_LUDO_BOARD_TXT: &[u8] = b"players=2
dice=1
position_a=0
position_b=0
";

const DEFAULT_SNAKES_APP_TOM: &[u8] = b"id = \"snakes_ladder\"
name = \"Snakes and Ladder\"
category = \"Games\"
type = \"activity\"
version = \"0.1.0\"
entry = \"MAIN.LUA\"
capabilities = [\"display\", \"input\", \"storage\"]
";
const DEFAULT_SNAKES_MAIN_LUA: &[u8] = b"-- Snakes and Ladder starter app
display_title = \"Snakes and Ladder\"
display_line1 = \"Starter Lua app installed by Rustmix.\"
display_line2 = \"Replace MAIN.LUA over Wi-Fi Transfer to customize.\"
display_footer = \"Folder: /RUSTMIX/APPS/SNAKES\"
";
const DEFAULT_SNAKES_BOARD_TXT: &[u8] = b"start=1
finish=100
snakes=16-6,48-30
ladders=3-22,8-26
";

const DEFAULT_APP_FILES: &[(&str, &str, &[u8])] = &[
    (
        "RUSTMIX/APPS/CALENDAR",
        "APP.TOM",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/CALENDAR/APP.TOM"
        )),
    ),
    (
        "RUSTMIX/APPS/CALENDAR",
        "MAIN.LUA",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/CALENDAR/MAIN.LUA"
        )),
    ),
    (
        "RUSTMIX/APPS/CALENDAR",
        "EVENTS.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/CALENDAR/EVENTS.TXT"
        )),
    ),
    (
        "RUSTMIX/APPS/CALENDAR",
        "US2026.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/CALENDAR/US2026.TXT"
        )),
    ),
    (
        "RUSTMIX/APPS/DICT",
        "APP.TOM",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/DICT/APP.TOM"
        )),
    ),
    (
        "RUSTMIX/APPS/DICT",
        "MAIN.LUA",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/DICT/MAIN.LUA"
        )),
    ),
    (
        "RUSTMIX/APPS/DICT",
        "DICT.JSN",
        DEFAULT_DICTIONARY_FALLBACK_JSN,
    ),
    (
        "RUSTMIX/APPS/DICT",
        "INDEX.TXT",
        DEFAULT_DICTIONARY_INDEX_TXT,
    ),
    ("RUSTMIX/APPS/DICT/DATA", "A.JSN", DEFAULT_DICTIONARY_A_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "B.JSN", DEFAULT_DICTIONARY_B_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "C.JSN", DEFAULT_DICTIONARY_C_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "D.JSN", DEFAULT_DICTIONARY_D_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "E.JSN", DEFAULT_DICTIONARY_E_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "F.JSN", DEFAULT_DICTIONARY_F_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "G.JSN", DEFAULT_DICTIONARY_G_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "H.JSN", DEFAULT_DICTIONARY_H_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "I.JSN", DEFAULT_DICTIONARY_I_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "J.JSN", DEFAULT_DICTIONARY_J_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "K.JSN", DEFAULT_DICTIONARY_K_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "L.JSN", DEFAULT_DICTIONARY_L_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "M.JSN", DEFAULT_DICTIONARY_M_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "N.JSN", DEFAULT_DICTIONARY_N_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "O.JSN", DEFAULT_DICTIONARY_O_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "P.JSN", DEFAULT_DICTIONARY_P_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "Q.JSN", DEFAULT_DICTIONARY_Q_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "S.JSN", DEFAULT_DICTIONARY_S_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "T.JSN", DEFAULT_DICTIONARY_T_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "U.JSN", DEFAULT_DICTIONARY_U_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "V.JSN", DEFAULT_DICTIONARY_V_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "W.JSN", DEFAULT_DICTIONARY_W_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "Y.JSN", DEFAULT_DICTIONARY_Y_JSN),
    ("RUSTMIX/APPS/DICT/DATA", "Z.JSN", DEFAULT_DICTIONARY_Z_JSN),
    (
        "RUSTMIX/APPS/DICT/DATA",
        "RUST.JSN",
        DEFAULT_DICTIONARY_RUST_JSN,
    ),
    (
        "RUSTMIX/APPS/DICT/DATA",
        "XTEINK.JSN",
        DEFAULT_DICTIONARY_XTEINK_JSN,
    ),
    (
        "RUSTMIX/APPS/FLASHCRD",
        "APP.TOM",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/FLASHCRD/APP.TOM"
        )),
    ),
    (
        "RUSTMIX/APPS/FLASHCRD",
        "MAIN.LUA",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/FLASHCRD/MAIN.LUA"
        )),
    ),
    (
        "RUSTMIX/APPS/FLASHCRD/TOPICS",
        "INDEX.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/FLASHCRD/TOPICS/INDEX.TXT"
        )),
    ),
    (
        "RUSTMIX/APPS/FLASHCRD/TOPICS/TEXTDEMO",
        "CARDS.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/FLASHCRD/TOPICS/TEXTDEMO/CARDS.TXT"
        )),
    ),
    (
        "RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO",
        "CARDS.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO/CARDS.TXT"
        )),
    ),
    (
        "RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO/IMG",
        "IMG01F.X4B",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO/IMG/IMG01F.X4B"
        )),
    ),
    (
        "RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO/IMG",
        "IMG01B.X4B",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO/IMG/IMG01B.X4B"
        )),
    ),
    (
        "RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO/IMG",
        "IMG02F.X4B",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO/IMG/IMG02F.X4B"
        )),
    ),
    (
        "RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO/IMG",
        "IMG02B.X4B",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/FLASHCRD/TOPICS/IMGDEMO/IMG/IMG02B.X4B"
        )),
    ),
    (
        "RUSTMIX/APPS/SUDOKU",
        "APP.TOM",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/SUDOKU/APP.TOM"
        )),
    ),
    (
        "RUSTMIX/APPS/SUDOKU",
        "MAIN.LUA",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/SUDOKU/MAIN.LUA"
        )),
    ),
    (
        "RUSTMIX/APPS/SUDOKU",
        "PUZZLES.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/SUDOKU/PUZZLES.TXT"
        )),
    ),
    (
        "RUSTMIX/APPS/MINES",
        "APP.TOM",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/MINES/APP.TOM"
        )),
    ),
    (
        "RUSTMIX/APPS/MINES",
        "MAIN.LUA",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/MINES/MAIN.LUA"
        )),
    ),
    (
        "RUSTMIX/APPS/MINES",
        "BOARD.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/MINES/BOARD.TXT"
        )),
    ),
    ("RUSTMIX/APPS/FREECELL", "APP.TOM", DEFAULT_FREECELL_APP_TOM),
    (
        "RUSTMIX/APPS/FREECELL",
        "MAIN.LUA",
        DEFAULT_FREECELL_MAIN_LUA,
    ),
    (
        "RUSTMIX/APPS/FREECELL",
        "CARDS.TXT",
        DEFAULT_FREECELL_CARDS_TXT,
    ),
    (
        "RUSTMIX/APPS/MEMCARD",
        "APP.TOM",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/MEMCARD/APP.TOM"
        )),
    ),
    (
        "RUSTMIX/APPS/MEMCARD",
        "MAIN.LUA",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/MEMCARD/MAIN.LUA"
        )),
    ),
    (
        "RUSTMIX/APPS/MEMCARD",
        "CARDS.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/MEMCARD/CARDS.TXT"
        )),
    ),
    ("RUSTMIX/APPS/SOLITAIR", "APP.TOM", DEFAULT_SOLITAIR_APP_TOM),
    (
        "RUSTMIX/APPS/SOLITAIR",
        "MAIN.LUA",
        DEFAULT_SOLITAIR_MAIN_LUA,
    ),
    (
        "RUSTMIX/APPS/SOLITAIR",
        "CARDS.TXT",
        DEFAULT_SOLITAIR_CARDS_TXT,
    ),
    ("RUSTMIX/APPS/LUDO", "APP.TOM", DEFAULT_LUDO_APP_TOM),
    ("RUSTMIX/APPS/LUDO", "MAIN.LUA", DEFAULT_LUDO_MAIN_LUA),
    ("RUSTMIX/APPS/LUDO", "BOARD.TXT", DEFAULT_LUDO_BOARD_TXT),
    ("RUSTMIX/APPS/SNAKES", "APP.TOM", DEFAULT_SNAKES_APP_TOM),
    ("RUSTMIX/APPS/SNAKES", "MAIN.LUA", DEFAULT_SNAKES_MAIN_LUA),
    ("RUSTMIX/APPS/SNAKES", "BOARD.TXT", DEFAULT_SNAKES_BOARD_TXT),
    (
        "RUSTMIX/APPS/UNITS",
        "APP.TOM",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/UNITS/APP.TOM"
        )),
    ),
    (
        "RUSTMIX/APPS/UNITS",
        "MAIN.LUA",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/UNITS/MAIN.LUA"
        )),
    ),
    (
        "RUSTMIX/APPS/UNITS",
        "UNITS.TXT",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/sd-card/RUSTMIX/APPS/UNITS/UNITS.TXT"
        )),
    ),
];

// backward-compatible alias
pub type StorageError = Error;

#[derive(Clone, Copy)]
pub struct DirEntry {
    pub name: [u8; 13],
    pub name_len: u8,
    pub is_dir: bool,
    pub size: u32,
    pub title: [u8; TITLE_CAP],
    pub title_len: u8,
}

impl DirEntry {
    pub const EMPTY: Self = Self {
        name: [0u8; 13],
        name_len: 0,
        is_dir: false,
        size: 0,
        title: [0u8; TITLE_CAP],
        title_len: 0,
    };

    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("?")
    }

    pub fn display_name(&self) -> &str {
        let len = (self.title_len & 0x7F) as usize;
        if len > 0 {
            core::str::from_utf8(&self.title[..len]).unwrap_or(self.name_str())
        } else {
            self.name_str()
        }
    }

    pub fn has_real_title(&self) -> bool {
        self.title_len > 0 && self.title_len & 0x80 == 0
    }

    pub fn set_title(&mut self, s: &[u8]) {
        let n = s.len().min(TITLE_CAP);
        self.title[..n].copy_from_slice(&s[..n]);
        self.title_len = n as u8;
    }

    // write a humanized SFN into the title buffer as a soft fallback;
    // does not prevent the title scanner from resolving a real title
    pub fn humanize_sfn(&mut self) {
        let nlen = self.name_len as usize;
        if nlen == 0 || self.has_real_title() {
            return;
        }
        let src = &self.name[..nlen];
        // check if name is all-uppercase (typical 8.3 SFN)
        let all_upper = src.iter().all(|&b| !b.is_ascii_lowercase());
        if !all_upper {
            return; // mixed case: user-supplied LFN, leave as-is
        }
        let n = nlen.min(TITLE_CAP);
        let dot_pos = src.iter().position(|&b| b == b'.').unwrap_or(n);
        for i in 0..n {
            if i == 0 {
                self.title[i] = src[i]; // keep first char uppercase
            } else if i > dot_pos {
                self.title[i] = src[i].to_ascii_lowercase(); // lowercase ext
            } else {
                self.title[i] = src[i].to_ascii_lowercase();
            }
        }
        self.title_len = 0x80 | n as u8;
    }
}

pub struct DirPage {
    pub total: usize,
    pub count: usize,
}

fn ext_eq(name: &[u8], target: &[u8]) -> bool {
    let dot = match name.iter().rposition(|&b| b == b'.') {
        Some(p) => p,
        None => return false,
    };
    let ext = &name[dot + 1..];
    ext.len() == target.len() && ext.eq_ignore_ascii_case(target)
}

fn has_supported_ext(name: &[u8]) -> bool {
    ext_eq(name, b"TXT") || ext_eq(name, b"EPUB") || ext_eq(name, b"EPU") || ext_eq(name, b"MD")
}

// build "NAME.EXT" bytes from a ShortFileName

fn sfn_to_bytes(name: &embedded_sdmmc::ShortFileName, out: &mut [u8; 13]) -> u8 {
    let base = name.base_name();
    let ext = name.extension();
    let mut pos = 0usize;
    let blen = base.len().min(8);
    out[..blen].copy_from_slice(&base[..blen]);
    pos += blen;
    if !ext.is_empty() {
        out[pos] = b'.';
        pos += 1;
        let elen = ext.len().min(3);
        out[pos..pos + elen].copy_from_slice(&ext[..elen]);
        pos += elen;
    }
    pos as u8
}

// file-operation macros; each evaluates to Result<T, Error>
// none use ? internally so caller cleanup is never bypassed

macro_rules! op_file_size {
    ($inner:expr, $dir:expr, $name:expr) => {
        $inner
            .mgr
            .find_directory_entry($dir, $name)
            .await
            .map(|e| e.size)
            .map_err(|_| Error::new(ErrorKind::OpenFile, "file_size"))
    };
}

macro_rules! op_read_chunk {
    ($inner:expr, $dir:expr, $name:expr, $offset:expr, $buf:expr) => {
        match $inner
            .mgr
            .open_file_in_dir($dir, $name, Mode::ReadOnly)
            .await
        {
            Err(_) => Err(Error::new(ErrorKind::OpenFile, "read_chunk")),
            Ok(file) => {
                let result = match $inner.mgr.file_seek_from_start(file, $offset) {
                    Ok(()) => $inner
                        .mgr
                        .read(file, $buf)
                        .await
                        .map_err(|_| Error::new(ErrorKind::ReadFailed, "read_chunk")),
                    Err(_) => Err(Error::new(ErrorKind::SeekFailed, "read_chunk")),
                };
                let _ = $inner.mgr.close_file(file).await;
                result
            }
        }
    };
}

macro_rules! op_read_start {
    ($inner:expr, $dir:expr, $name:expr, $buf:expr) => {
        match $inner
            .mgr
            .open_file_in_dir($dir, $name, Mode::ReadOnly)
            .await
        {
            Err(_) => Err(Error::new(ErrorKind::OpenFile, "read_start")),
            Ok(file) => {
                let size = $inner.mgr.file_length(file).unwrap_or(0);
                let result = $inner
                    .mgr
                    .read(file, $buf)
                    .await
                    .map_err(|_| Error::new(ErrorKind::ReadFailed, "read_start"));
                let _ = $inner.mgr.close_file(file).await;
                result.map(|n| (size, n))
            }
        }
    };
}

macro_rules! op_write {
    ($inner:expr, $dir:expr, $name:expr, $data:expr) => {
        match $inner
            .mgr
            .open_file_in_dir($dir, $name, Mode::ReadWriteCreateOrTruncate)
            .await
        {
            Err(_) => Err(Error::new(ErrorKind::OpenFile, "write")),
            Ok(file) => {
                let result = if ($data).is_empty() {
                    Ok(())
                } else {
                    $inner
                        .mgr
                        .write(file, $data)
                        .await
                        .map_err(|_| Error::new(ErrorKind::WriteFailed, "write"))
                };
                let _ = $inner.mgr.close_file(file).await;
                result
            }
        }
    };
}

macro_rules! op_append {
    ($inner:expr, $dir:expr, $name:expr, $data:expr) => {
        match $inner
            .mgr
            .open_file_in_dir($dir, $name, Mode::ReadWriteCreateOrAppend)
            .await
        {
            Err(_) => Err(Error::new(ErrorKind::OpenFile, "append")),
            Ok(file) => {
                let result = if ($data).is_empty() {
                    Ok(())
                } else {
                    $inner
                        .mgr
                        .write(file, $data)
                        .await
                        .map_err(|_| Error::new(ErrorKind::WriteFailed, "append"))
                };
                let _ = $inner.mgr.close_file(file).await;
                result
            }
        }
    };
}

macro_rules! op_delete {
    ($inner:expr, $dir:expr, $name:expr) => {{
        $inner
            .mgr
            .delete_entry_in_dir($dir, $name)
            .await
            .map_err(|_| Error::new(ErrorKind::DeleteFailed, "delete"))
    }};
}

// dir-scoping macros; open subdir, execute body, close handle

macro_rules! in_dir {
    ($inner:expr, $dirname:expr, |$dir:ident| $body:expr) => {
        match $inner.mgr.open_dir($inner.root, $dirname).await {
            Err(_) => Err(Error::new(ErrorKind::OpenDir, "in_dir")),
            Ok($dir) => {
                let _r = $body;
                let _ = $inner.mgr.close_dir($dir);
                _r
            }
        }
    };
}

macro_rules! in_subdir {
    ($inner:expr, $d1:expr, $d2:expr, |$dir:ident| $body:expr) => {
        match $inner.mgr.open_dir($inner.root, $d1).await {
            Err(_) => Err(Error::new(ErrorKind::OpenDir, "in_subdir")),
            Ok(_mid) => match $inner.mgr.open_dir(_mid, $d2).await {
                Err(_) => {
                    let _ = $inner.mgr.close_dir(_mid);
                    Err(Error::new(ErrorKind::OpenDir, "in_subdir"))
                }
                Ok($dir) => {
                    let _r = $body;
                    let _ = $inner.mgr.close_dir($dir);
                    let _ = $inner.mgr.close_dir(_mid);
                    _r
                }
            },
        }
    };
}

fn borrow(sd: &SdStorage) -> core::result::Result<core::cell::RefMut<'_, SdStorageInner>, Error> {
    sd.borrow_inner()
        .ok_or(Error::new(ErrorKind::NoCard, "storage::borrow"))
}

fn validate_nested_storage_path(path: &str) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    if path.starts_with('/') || path.ends_with('/') || path.contains("//") {
        return Err(Error::new(ErrorKind::InvalidData, "nested_path_shape"));
    }

    let mut count = 0usize;
    for component in path.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component
                .as_bytes()
                .iter()
                .any(|&b| b == b'\\' || b == b':')
        {
            return Err(Error::new(ErrorKind::InvalidData, "nested_path_component"));
        }
        count += 1;
        if count > MAX_NESTED_STORAGE_PATH_COMPONENTS {
            return Err(Error::new(ErrorKind::InvalidData, "nested_path_depth"));
        }
    }

    Ok(())
}

async fn open_nested_storage_dir(
    inner: &mut SdStorageInner,
    path: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<(RawDirectory, bool)> {
    if path.is_empty() {
        return Ok((inner.root, false));
    }
    validate_nested_storage_path(path)?;

    let mut current = inner.root;
    let mut current_is_child = false;
    let mut depth = 0usize;

    for component in path.split('/') {
        if depth >= MAX_NESTED_STORAGE_PATH_COMPONENTS {
            close_nested_storage_dir(inner, current, current_is_child);
            return Err(Error::new(
                ErrorKind::OpenDir,
                "open_nested_storage_dir_depth",
            ));
        }

        match inner.mgr.open_dir(current, component).await {
            Ok(next) => {
                close_nested_storage_dir(inner, current, current_is_child);
                current = next;
                current_is_child = true;
                depth += 1;
            }
            Err(_) => {
                close_nested_storage_dir(inner, current, current_is_child);
                return Err(Error::new(ErrorKind::OpenDir, "open_nested_storage_dir"));
            }
        }
    }

    Ok((current, current_is_child))
}

fn close_nested_storage_dir(inner: &mut SdStorageInner, dir: RawDirectory, should_close: bool) {
    if should_close {
        let _ = inner.mgr.close_dir(dir);
    }
}

// root file operations

pub fn file_size(sd: &SdStorage, name: &str) -> crate::rustmix_x4::x4_kernel::error::Result<u32> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        op_file_size!(inner, inner.root, name)
    })
}

pub fn read_file_chunk(
    sd: &SdStorage,
    name: &str,
    offset: u32,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        op_read_chunk!(inner, inner.root, name, offset, buf)
    })
}

pub fn read_file_start(
    sd: &SdStorage,
    name: &str,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        op_read_start!(inner, inner.root, name, buf)
    })
}

pub fn write_file(
    sd: &SdStorage,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        op_write!(inner, inner.root, name, data)
    })
}

pub fn append_root_file(
    sd: &SdStorage,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        op_append!(inner, inner.root, name, data)
    })
}

pub fn delete_file(sd: &SdStorage, name: &str) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        op_delete!(inner, inner.root, name)
    })
}

// directory listing

pub fn list_root_files(
    sd: &SdStorage,
    buf: &mut [DirEntry],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        let mut count = 0usize;
        let mut total = 0usize;

        inner
            .mgr
            .iterate_dir(inner.root, |entry| {
                if entry.attributes.is_volume() || entry.attributes.is_directory() {
                    return ControlFlow::Continue(());
                }

                let mut name_buf = [0u8; 13];
                let name_len = sfn_to_bytes(&entry.name, &mut name_buf);
                let sfn = &name_buf[..name_len as usize];

                if sfn.is_empty() || sfn[0] == b'.' || sfn[0] == b'_' {
                    return ControlFlow::Continue(());
                }
                if !has_supported_ext(sfn) {
                    return ControlFlow::Continue(());
                }

                total += 1;

                if count < buf.len() {
                    buf[count] = DirEntry {
                        name: name_buf,
                        name_len,
                        is_dir: false,
                        size: entry.size,
                        title: [0u8; TITLE_CAP],
                        title_len: 0,
                    };
                    count += 1;
                }
                ControlFlow::Continue(())
            })
            .await
            .map_err(|_| Error::new(ErrorKind::ReadFailed, "list_root_files"))?;

        if total > count {
            log::warn!(
                "dir: {} supported files on SD, only {} fit in buffer (max {})",
                total,
                count,
                buf.len(),
            );
        }
        Ok(count)
    })
}

pub fn list_root_entries(
    sd: &SdStorage,
    buf: &mut [DirEntry],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        let mut count = 0usize;
        let mut total = 0usize;

        inner
            .mgr
            .iterate_dir(inner.root, |entry| {
                if entry.attributes.is_volume() {
                    return ControlFlow::Continue(());
                }

                let mut name_buf = [0u8; 13];
                let name_len = sfn_to_bytes(&entry.name, &mut name_buf);
                let sfn = &name_buf[..name_len as usize];

                if sfn.is_empty() || sfn[0] == b'.' {
                    return ControlFlow::Continue(());
                }

                total += 1;

                if count < buf.len() {
                    buf[count] = DirEntry {
                        name: name_buf,
                        name_len,
                        is_dir: entry.attributes.is_directory(),
                        size: entry.size,
                        title: [0u8; TITLE_CAP],
                        title_len: 0,
                    };
                    count += 1;
                }

                ControlFlow::Continue(())
            })
            .await
            .map_err(|_| Error::new(ErrorKind::ReadFailed, "list_root_entries"))?;

        if total > count {
            log::warn!(
                "sd-manager: {} entries on SD, only {} fit in buffer",
                total,
                count
            );
        }

        Ok(count)
    })
}

pub fn list_dir_entries(
    sd: &SdStorage,
    dir_name: &str,
    buf: &mut [DirEntry],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        let dir = match inner.mgr.open_dir(inner.root, dir_name).await {
            Ok(d) => d,
            Err(_) => return Err(Error::new(ErrorKind::OpenDir, "list_dir_entries")),
        };

        let mut count = 0usize;
        let mut total = 0usize;

        let result = inner
            .mgr
            .iterate_dir(dir, |entry| {
                if entry.attributes.is_volume() {
                    return ControlFlow::Continue(());
                }

                let mut name_buf = [0u8; 13];
                let name_len = sfn_to_bytes(&entry.name, &mut name_buf);
                let sfn = &name_buf[..name_len as usize];

                if sfn.is_empty() || sfn[0] == b'.' {
                    return ControlFlow::Continue(());
                }

                total += 1;

                if count < buf.len() {
                    buf[count] = DirEntry {
                        name: name_buf,
                        name_len,
                        is_dir: entry.attributes.is_directory(),
                        size: entry.size,
                        title: [0u8; TITLE_CAP],
                        title_len: 0,
                    };
                    count += 1;
                }

                ControlFlow::Continue(())
            })
            .await
            .map_err(|_| Error::new(ErrorKind::ReadFailed, "list_dir_entries"))
            .map(|_| count);

        let _ = inner.mgr.close_dir(dir);

        if result.is_ok() && total > count {
            log::warn!(
                "sd-manager: {} entries in {}, only {} fit in buffer",
                total,
                dir_name,
                count
            );
        }

        result
    })
}

pub fn delete_file_in_dir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, dir, |dir_h| op_delete!(inner, dir_h, name))
    })
}

pub fn list_subdir_entries(
    sd: &SdStorage,
    dir: &str,
    subdir: &str,
    buf: &mut [DirEntry],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        in_subdir!(inner, dir, subdir, |dir_h| {
            let mut count = 0usize;
            let mut total = 0usize;

            let result = inner
                .mgr
                .iterate_dir(dir_h, |entry| {
                    if entry.attributes.is_volume() {
                        return ControlFlow::Continue(());
                    }

                    let mut name_buf = [0u8; 13];
                    let name_len = sfn_to_bytes(&entry.name, &mut name_buf);
                    let sfn = &name_buf[..name_len as usize];

                    if sfn.is_empty() || sfn[0] == b'.' {
                        return ControlFlow::Continue(());
                    }

                    total += 1;

                    if count < buf.len() {
                        buf[count] = DirEntry {
                            name: name_buf,
                            name_len,
                            is_dir: entry.attributes.is_directory(),
                            size: entry.size,
                            title: [0u8; TITLE_CAP],
                            title_len: 0,
                        };
                        count += 1;
                    }

                    ControlFlow::Continue(())
                })
                .await
                .map_err(|_| Error::new(ErrorKind::ReadFailed, "list_subdir_entries"))
                .map(|_| count);

            if result.is_ok() && total > count {
                log::warn!(
                    "sd-manager: {} entries in {}/{}, only {} fit in buffer",
                    total,
                    dir,
                    subdir,
                    count
                );
            }

            result
        })
    })
}

pub fn list_path_entries(
    sd: &SdStorage,
    path: &str,
    buf: &mut [DirEntry],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let (dir, should_close) = open_nested_storage_dir(inner, path).await?;

        let mut count = 0usize;
        let mut total = 0usize;
        let result = inner
            .mgr
            .iterate_dir(dir, |entry| {
                if entry.attributes.is_volume() {
                    return ControlFlow::Continue(());
                }

                let mut name_buf = [0u8; 13];
                let name_len = sfn_to_bytes(&entry.name, &mut name_buf);
                let sfn = &name_buf[..name_len as usize];
                if sfn.is_empty() || sfn[0] == b'.' {
                    return ControlFlow::Continue(());
                }

                total += 1;
                if count < buf.len() {
                    buf[count] = DirEntry {
                        name: name_buf,
                        name_len,
                        is_dir: entry.attributes.is_directory(),
                        size: entry.size,
                        title: [0u8; TITLE_CAP],
                        title_len: 0,
                    };
                    count += 1;
                }

                ControlFlow::Continue(())
            })
            .await
            .map_err(|_| Error::new(ErrorKind::ReadFailed, "list_path_entries"))
            .map(|_| count);

        close_nested_storage_dir(inner, dir, should_close);

        if result.is_ok() && total > count {
            log::warn!(
                "sd-manager: {} entries in {}, only {} fit in buffer",
                total,
                path,
                count
            );
        }

        result
    })
}

pub fn ensure_path(sd: &SdStorage, path: &str) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    let path = path.trim_matches('/');
    if path.is_empty() {
        return Ok(());
    }
    validate_nested_storage_path(path)?;

    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let mut current = inner.root;
        let mut current_is_child = false;
        let mut depth = 0usize;

        for component in path.split('/') {
            if depth >= MAX_NESTED_STORAGE_PATH_COMPONENTS {
                close_nested_storage_dir(inner, current, current_is_child);
                return Err(Error::new(ErrorKind::WriteFailed, "ensure_path_depth"));
            }

            let child = match inner.mgr.open_dir(current, component).await {
                Ok(child) => child,
                Err(_) => {
                    match inner.mgr.make_dir_in_dir(current, component).await {
                        Ok(()) | Err(embedded_sdmmc::Error::DirAlreadyExists) => {}
                        Err(_) => {
                            close_nested_storage_dir(inner, current, current_is_child);
                            return Err(Error::new(ErrorKind::WriteFailed, "ensure_path"));
                        }
                    }
                    match inner.mgr.open_dir(current, component).await {
                        Ok(child) => child,
                        Err(_) => {
                            close_nested_storage_dir(inner, current, current_is_child);
                            return Err(Error::new(ErrorKind::OpenDir, "ensure_path_open"));
                        }
                    }
                }
            };

            close_nested_storage_dir(inner, current, current_is_child);
            current = child;
            current_is_child = true;
            depth += 1;
        }

        close_nested_storage_dir(inner, current, current_is_child);
        Ok(())
    })
}

pub fn file_size_in_path(
    sd: &SdStorage,
    path: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<u32> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let (dir, should_close) = open_nested_storage_dir(inner, path).await?;
        let result = op_file_size!(inner, dir, name);
        close_nested_storage_dir(inner, dir, should_close);
        result
    })
}

pub fn read_file_start_in_path(
    sd: &SdStorage,
    path: &str,
    name: &str,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let (dir, should_close) = open_nested_storage_dir(inner, path).await?;
        let result = op_read_start!(inner, dir, name, buf);
        close_nested_storage_dir(inner, dir, should_close);
        result
    })
}

pub fn read_file_chunk_in_path(
    sd: &SdStorage,
    path: &str,
    name: &str,
    offset: u32,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let (dir, should_close) = open_nested_storage_dir(inner, path).await?;
        let result = op_read_chunk!(inner, dir, name, offset, buf);
        close_nested_storage_dir(inner, dir, should_close);
        result
    })
}

pub fn write_file_in_path(
    sd: &SdStorage,
    path: &str,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let (dir, should_close) = open_nested_storage_dir(inner, path).await?;
        let result = op_write!(inner, dir, name, data);
        close_nested_storage_dir(inner, dir, should_close);
        result
    })
}

pub fn append_file_in_path(
    sd: &SdStorage,
    path: &str,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let (dir, should_close) = open_nested_storage_dir(inner, path).await?;
        let result = op_append!(inner, dir, name, data);
        close_nested_storage_dir(inner, dir, should_close);
        result
    })
}

pub fn delete_file_in_path(
    sd: &SdStorage,
    path: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let (dir, should_close) = open_nested_storage_dir(inner, path).await?;
        let result = op_delete!(inner, dir, name);
        close_nested_storage_dir(inner, dir, should_close);
        result
    })
}

// directory management

pub fn ensure_dir(sd: &SdStorage, name: &str) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    // two poll_once calls so the large make_dir future never shares
    // a stack frame with open_dir, halving peak stack usage
    let exists = poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        match inner.mgr.open_dir(inner.root, name).await {
            Ok(dir) => {
                let _ = inner.mgr.close_dir(dir);
                Ok::<_, Error>(true)
            }
            Err(_) => Ok(false),
        }
    })?;

    if exists {
        return Ok(());
    }

    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        match inner.mgr.make_dir_in_dir(inner.root, name).await {
            Ok(()) => Ok(()),
            Err(embedded_sdmmc::Error::DirAlreadyExists) => Ok(()),
            Err(_) => Err(Error::new(ErrorKind::WriteFailed, "ensure_dir")),
        }
    })
}

// single-directory file operations

pub fn write_file_in_dir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, dir, |dir_h| op_write!(inner, dir_h, name, data))
    })
}

pub fn append_file_in_dir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, dir, |dir_h| op_append!(inner, dir_h, name, data))
    })
}

pub fn file_size_in_dir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<u32> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, dir, |dir_h| op_file_size!(inner, dir_h, name))
    })
}

pub fn file_size_in_subdir(
    sd: &SdStorage,
    dir: &str,
    subdir: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<u32> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, dir, subdir, |dir_h| op_file_size!(
            inner, dir_h, name
        ))
    })
}

pub fn read_file_chunk_in_dir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
    offset: u32,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, dir, |dir_h| op_read_chunk!(
            inner, dir_h, name, offset, buf
        ))
    })
}

pub fn read_file_start_in_dir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, dir, |dir_h| op_read_start!(inner, dir_h, name, buf))
    })
}

pub fn read_file_chunk_in_subdir(
    sd: &SdStorage,
    dir: &str,
    subdir: &str,
    name: &str,
    offset: u32,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, dir, subdir, |dir_h| op_read_chunk!(
            inner, dir_h, name, offset, buf
        ))
    })
}

pub fn read_file_start_in_subdir(
    sd: &SdStorage,
    dir: &str,
    subdir: &str,
    name: &str,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, dir, subdir, |dir_h| op_read_start!(
            inner, dir_h, name, buf
        ))
    })
}

/// Read the start of a file under a fixed three-level directory path.
///
/// This is used by the first Lua app proof to read
/// `/RUSTMIX/APPS/<app_id>/MAIN.LUA` without adding recursive SD scanning or
/// changing raw SD/FAT/SPI behavior.
pub fn read_file_start_in_three_subdir(
    sd: &SdStorage,
    dir1: &str,
    dir2: &str,
    dir3: &str,
    name: &str,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let d1 = inner
            .mgr
            .open_dir(inner.root, dir1)
            .await
            .map_err(|_| Error::new(ErrorKind::OpenDir, "read_file_start_in_three_subdir"))?;
        let d2 = match inner.mgr.open_dir(d1, dir2).await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(d1);
                return Err(Error::new(
                    ErrorKind::OpenDir,
                    "read_file_start_in_three_subdir",
                ));
            }
        };
        let d3 = match inner.mgr.open_dir(d2, dir3).await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(d2);
                let _ = inner.mgr.close_dir(d1);
                return Err(Error::new(
                    ErrorKind::OpenDir,
                    "read_file_start_in_three_subdir",
                ));
            }
        };

        let result = op_read_start!(inner, d3, name, buf);
        let _ = inner.mgr.close_dir(d3);
        let _ = inner.mgr.close_dir(d2);
        let _ = inner.mgr.close_dir(d1);
        result
    })
}

/// Read the start of a file under a fixed four-level directory path.
///
/// This is used by SD-loaded Lua apps that keep app data one level below
/// the physical app folder, for example:
/// `/RUSTMIX/APPS/PANCHANG/DATA/Y2026.TXT`.
///
/// It intentionally opens a fixed-depth path and does not add recursive SD
/// scanning or change raw SD/FAT/SPI behavior.
pub fn read_file_start_in_four_subdir(
    sd: &SdStorage,
    dir1: &str,
    dir2: &str,
    dir3: &str,
    dir4: &str,
    name: &str,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        let d1 = inner
            .mgr
            .open_dir(inner.root, dir1)
            .await
            .map_err(|_| Error::new(ErrorKind::OpenDir, "read_file_start_in_four_subdir"))?;
        let d2 = match inner.mgr.open_dir(d1, dir2).await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(d1);
                return Err(Error::new(
                    ErrorKind::OpenDir,
                    "read_file_start_in_four_subdir",
                ));
            }
        };
        let d3 = match inner.mgr.open_dir(d2, dir3).await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(d2);
                let _ = inner.mgr.close_dir(d1);
                return Err(Error::new(
                    ErrorKind::OpenDir,
                    "read_file_start_in_four_subdir",
                ));
            }
        };
        let d4 = match inner.mgr.open_dir(d3, dir4).await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(d3);
                let _ = inner.mgr.close_dir(d2);
                let _ = inner.mgr.close_dir(d1);
                return Err(Error::new(
                    ErrorKind::OpenDir,
                    "read_file_start_in_four_subdir",
                ));
            }
        };

        let result = op_read_start!(inner, d4, name, buf);
        let _ = inner.mgr.close_dir(d4);
        let _ = inner.mgr.close_dir(d3);
        let _ = inner.mgr.close_dir(d2);
        let _ = inner.mgr.close_dir(d1);
        result
    })
}

/// Read the start of a file under the canonical Lua app data path:
/// `/RUSTMIX/APPS/<APP>/DATA/<NAME>`.
///
/// This helper is intentionally fixed-depth and read-only. It does not add
/// recursive SD scanning, does not change raw SD/FAT/SPI behavior, and exists
/// only to avoid treating `DATA/Y2026.TXT` as a single 8.3 filename.
pub fn read_file_start_in_rustmix_lua_app_data_file(
    sd: &SdStorage,
    app_folder: &str,
    data_dir: &str,
    name: &str,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        let rustmix = inner
            .mgr
            .open_dir(inner.root, "RUSTMIX")
            .await
            .map_err(|_| Error::new(ErrorKind::OpenDir, "lua_app_data:RUSTMIX"))?;

        let apps = match inner.mgr.open_dir(rustmix, "APPS").await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(rustmix);
                return Err(Error::new(ErrorKind::OpenDir, "lua_app_data:APPS"));
            }
        };

        let app = match inner.mgr.open_dir(apps, app_folder).await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(apps);
                let _ = inner.mgr.close_dir(rustmix);
                return Err(Error::new(ErrorKind::OpenDir, "lua_app_data:APP"));
            }
        };

        let data = match inner.mgr.open_dir(app, data_dir).await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(app);
                let _ = inner.mgr.close_dir(apps);
                let _ = inner.mgr.close_dir(rustmix);
                return Err(Error::new(ErrorKind::OpenDir, "lua_app_data:DATA"));
            }
        };

        let result = op_read_start!(inner, data, name, buf);

        let _ = inner.mgr.close_dir(data);
        let _ = inner.mgr.close_dir(app);
        let _ = inner.mgr.close_dir(apps);
        let _ = inner.mgr.close_dir(rustmix);

        result
    })
}

/// Read `/RUSTMIX/APPS/PANCHANG/DATA/Y2026.TXT` using explicit path segments.
///
/// This intentionally avoids generic slash-containing file names and avoids
/// recursive scanning. It mirrors the path that Wi-Fi Transfer lists on the SD
/// card and is used only by the Lua Panchang runtime app.
pub fn read_rustmix_apps_panchang_y2026_start(
    sd: &SdStorage,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        let rustmix = inner
            .mgr
            .open_dir(inner.root, "RUSTMIX")
            .await
            .map_err(|_| Error::new(ErrorKind::OpenDir, "panchang_y2026:RUSTMIX"))?;

        let apps = match inner.mgr.open_dir(rustmix, "APPS").await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(rustmix);
                return Err(Error::new(ErrorKind::OpenDir, "panchang_y2026:APPS"));
            }
        };

        let panchang = match inner.mgr.open_dir(apps, "PANCHANG").await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(apps);
                let _ = inner.mgr.close_dir(rustmix);
                return Err(Error::new(ErrorKind::OpenDir, "panchang_y2026:PANCHANG"));
            }
        };

        let data = match inner.mgr.open_dir(panchang, "DATA").await {
            Ok(dir) => dir,
            Err(_) => {
                let _ = inner.mgr.close_dir(panchang);
                let _ = inner.mgr.close_dir(apps);
                let _ = inner.mgr.close_dir(rustmix);
                return Err(Error::new(ErrorKind::OpenDir, "panchang_y2026:DATA"));
            }
        };

        let result = op_read_start!(inner, data, "Y2026.TXT", buf);

        let _ = inner.mgr.close_dir(data);
        let _ = inner.mgr.close_dir(panchang);
        let _ = inner.mgr.close_dir(apps);
        let _ = inner.mgr.close_dir(rustmix);

        result
    })
}

pub fn ensure_dir_in_dir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    let exists = poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        in_dir!(inner, dir, |dir_h| {
            match inner.mgr.open_dir(dir_h, name).await {
                Ok(child) => {
                    let _ = inner.mgr.close_dir(child);
                    Ok::<_, Error>(true)
                }
                Err(_) => Ok(false),
            }
        })
    })?;

    if exists {
        return Ok(());
    }

    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        in_dir!(inner, dir, |dir_h| {
            match inner.mgr.make_dir_in_dir(dir_h, name).await {
                Ok(()) => Ok(()),
                Err(embedded_sdmmc::Error::DirAlreadyExists) => Ok(()),
                Err(_) => Err(Error::new(ErrorKind::WriteFailed, "ensure_dir_in_dir")),
            }
        })
    })
}

pub fn ensure_dir_in_subdir(
    sd: &SdStorage,
    dir: &str,
    subdir: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    let exists = poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        in_subdir!(inner, dir, subdir, |dir_h| {
            match inner.mgr.open_dir(dir_h, name).await {
                Ok(child) => {
                    let _ = inner.mgr.close_dir(child);
                    Ok::<_, Error>(true)
                }
                Err(_) => Ok(false),
            }
        })
    })?;

    if exists {
        return Ok(());
    }

    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;

        in_subdir!(inner, dir, subdir, |dir_h| {
            match inner.mgr.make_dir_in_dir(dir_h, name).await {
                Ok(()) => Ok(()),
                Err(embedded_sdmmc::Error::DirAlreadyExists) => Ok(()),
                Err(_) => Err(Error::new(ErrorKind::WriteFailed, "ensure_dir_in_subdir")),
            }
        })
    })
}

pub fn write_file_in_subdir(
    sd: &SdStorage,
    dir: &str,
    subdir: &str,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, dir, subdir, |dir_h| op_write!(
            inner, dir_h, name, data
        ))
    })
}

pub fn append_file_in_subdir(
    sd: &SdStorage,
    dir: &str,
    subdir: &str,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, dir, subdir, |dir_h| op_append!(
            inner, dir_h, name, data
        ))
    })
}

pub fn delete_file_in_subdir(
    sd: &SdStorage,
    dir: &str,
    subdir: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, dir, subdir, |dir_h| op_delete!(inner, dir_h, name))
    })
}

// Rustmix first-boot SD provisioning. This seeds only missing files and never
// overwrites user data, so normal app-only firmware flashes preserve local
// settings, apps, fonts, books, cache, and flashcard topics.
pub fn provision_rustmix_sd_defaults(
    sd: &SdStorage,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    // Root failure means the SD card cannot be provisioned at all.
    ensure_path(sd, RUSTMIX_DIR)?;

    // After the root exists, provisioning is deliberately best-effort. A bad
    // font file or one app data file must not prevent the remaining starter
    // apps from being installed on first boot. This also lets later firmware
    // flashes add newly introduced defaults without touching user files.
    let mut attempted = 0usize;
    let mut written_or_existing = 0usize;

    for path in [
        "RUSTMIX/APPS",
        "RUSTMIX/FONTS",
        "RUSTMIX/SLEEP",
        "RUSTMIX/CACHE",
        "RUSTMIX/STATE",
    ] {
        attempted += 1;
        if ensure_path_best_effort(sd, path) {
            written_or_existing += 1;
        }
    }

    for (path, name, data) in [
        (RUSTMIX_DIR, "SETTINGS.TXT", DEFAULT_SETTINGS_TXT),
        (RUSTMIX_DIR, "TIME.TXT", DEFAULT_TIME_TXT),
        (RUSTMIX_DIR, "PROVISION.TXT", DEFAULT_PROVISION_TXT),
    ] {
        attempted += 1;
        if seed_path_file_if_missing(sd, path, name, data) {
            written_or_existing += 1;
        }
    }

    for (name, data) in DEFAULT_FONTS.iter().copied() {
        attempted += 1;
        if seed_path_file_if_missing(sd, "RUSTMIX/FONTS", name, data) {
            written_or_existing += 1;
        }
    }

    for (name, data) in DEFAULT_SLEEP_FILES.iter().copied() {
        attempted += 1;
        if seed_or_repair_sleep_file(sd, "RUSTMIX/SLEEP", name, data) {
            written_or_existing += 1;
        }
    }

    for (path, name, data) in DEFAULT_APP_FILES.iter().copied() {
        attempted += 1;
        let dir_ready = ensure_path_best_effort(sd, path);
        if dir_ready && seed_path_file_if_missing(sd, path, name, data) {
            written_or_existing += 1;
        }
    }

    attempted += 1;
    if repair_default_flashcards_index(sd) {
        written_or_existing += 1;
    }

    attempted += 1;
    if repair_default_dictionary_index(sd) {
        written_or_existing += 1;
    }

    attempted += 1;
    if repair_compact_dictionary_seed(sd) {
        written_or_existing += 1;
    }

    log::info!(
        "rustmix-provision: attempted={} ok={} policy=seed-missing-files-only flashcards=text-image-defaults",
        attempted,
        written_or_existing
    );

    Ok(())
}

fn repair_default_flashcards_index(sd: &SdStorage) -> bool {
    let path = "RUSTMIX/APPS/FLASHCRD/TOPICS";
    let mut ok = true;
    let mut buf = [0u8; 768];
    let current = match read_file_start_in_path(sd, path, "INDEX.TXT", &mut buf) {
        Ok((_, n)) => core::str::from_utf8(&buf[..n]).unwrap_or(""),
        Err(_) => "",
    };

    if current.is_empty() {
        return match write_file_in_path(
            sd,
            path,
            "INDEX.TXT",
            b"TEXTDEMO|Text Flashcards|TEXT\nIMGDEMO|Image Flashcards|IMAGE\n",
        ) {
            Ok(()) => {
                log::info!("rustmix-provision: seeded {}/INDEX.TXT", path);
                true
            }
            Err(e) => {
                log::warn!("rustmix-provision: seed {}/INDEX.TXT failed: {:?}", path, e);
                false
            }
        };
    }

    if !current.contains("TEXTDEMO|") {
        match append_file_in_path(sd, path, "INDEX.TXT", b"\nTEXTDEMO|Text Flashcards|TEXT\n") {
            Ok(()) => log::info!("rustmix-provision: appended TEXTDEMO flashcard topic"),
            Err(e) => {
                log::warn!("rustmix-provision: append TEXTDEMO failed: {:?}", e);
                ok = false;
            }
        }
    }

    if !current.contains("IMGDEMO|") {
        match append_file_in_path(sd, path, "INDEX.TXT", b"\nIMGDEMO|Image Flashcards|IMAGE\n") {
            Ok(()) => log::info!("rustmix-provision: appended IMGDEMO flashcard topic"),
            Err(e) => {
                log::warn!("rustmix-provision: append IMGDEMO failed: {:?}", e);
                ok = false;
            }
        }
    }

    ok
}

fn repair_default_dictionary_index(sd: &SdStorage) -> bool {
    let path = "RUSTMIX/APPS/DICT";
    let mut buf = [0u8; 512];
    let needs_repair = match read_file_start_in_path(sd, path, "INDEX.TXT", &mut buf) {
        Ok((_, n)) => match core::str::from_utf8(&buf[..n]) {
            Ok(text) => !text.contains("A|DATA/A.JSN") || !text.contains("Z|DATA/Z.JSN"),
            Err(_) => true,
        },
        Err(_) => true,
    };

    if !needs_repair {
        return true;
    }

    match write_file_in_path(sd, path, "INDEX.TXT", DEFAULT_DICTIONARY_INDEX_TXT) {
        Ok(()) => {
            log::info!("rustmix-provision: repaired {}/INDEX.TXT", path);
            true
        }
        Err(e) => {
            log::warn!(
                "rustmix-provision: repair {}/INDEX.TXT failed: {:?}",
                path,
                e
            );
            false
        }
    }
}

fn repair_compact_dictionary_seed(sd: &SdStorage) -> bool {
    let mut ok = true;

    ok &= ensure_path_best_effort(sd, "RUSTMIX/APPS/DICT");
    ok &= ensure_path_best_effort(sd, "RUSTMIX/APPS/DICT/DATA");

    if dictionary_seed_needs_repair(sd, "RUSTMIX/APPS/DICT", "DICT.JSN") {
        ok &= write_dictionary_seed(
            sd,
            "RUSTMIX/APPS/DICT",
            "DICT.JSN",
            DEFAULT_DICTIONARY_FALLBACK_JSN,
        );
    }

    for (name, data) in [
        ("A.JSN", DEFAULT_DICTIONARY_A_JSN),
        ("B.JSN", DEFAULT_DICTIONARY_B_JSN),
        ("C.JSN", DEFAULT_DICTIONARY_C_JSN),
        ("D.JSN", DEFAULT_DICTIONARY_D_JSN),
        ("E.JSN", DEFAULT_DICTIONARY_E_JSN),
        ("F.JSN", DEFAULT_DICTIONARY_F_JSN),
        ("G.JSN", DEFAULT_DICTIONARY_G_JSN),
        ("H.JSN", DEFAULT_DICTIONARY_H_JSN),
        ("I.JSN", DEFAULT_DICTIONARY_I_JSN),
        ("J.JSN", DEFAULT_DICTIONARY_J_JSN),
        ("K.JSN", DEFAULT_DICTIONARY_K_JSN),
        ("L.JSN", DEFAULT_DICTIONARY_L_JSN),
        ("M.JSN", DEFAULT_DICTIONARY_M_JSN),
        ("N.JSN", DEFAULT_DICTIONARY_N_JSN),
        ("O.JSN", DEFAULT_DICTIONARY_O_JSN),
        ("P.JSN", DEFAULT_DICTIONARY_P_JSN),
        ("Q.JSN", DEFAULT_DICTIONARY_Q_JSN),
        ("S.JSN", DEFAULT_DICTIONARY_S_JSN),
        ("T.JSN", DEFAULT_DICTIONARY_T_JSN),
        ("U.JSN", DEFAULT_DICTIONARY_U_JSN),
        ("V.JSN", DEFAULT_DICTIONARY_V_JSN),
        ("W.JSN", DEFAULT_DICTIONARY_W_JSN),
        ("Y.JSN", DEFAULT_DICTIONARY_Y_JSN),
        ("Z.JSN", DEFAULT_DICTIONARY_Z_JSN),
        ("RUST.JSN", DEFAULT_DICTIONARY_RUST_JSN),
        ("XTEINK.JSN", DEFAULT_DICTIONARY_XTEINK_JSN),
    ] {
        if dictionary_seed_needs_repair(sd, "RUSTMIX/APPS/DICT/DATA", name) {
            ok &= write_dictionary_seed(sd, "RUSTMIX/APPS/DICT/DATA", name, data);
        }
    }

    ok
}

fn dictionary_seed_needs_repair(sd: &SdStorage, path: &str, name: &str) -> bool {
    let mut buf = [0u8; 48];
    match read_file_start_in_path(sd, path, name, &mut buf) {
        Ok((size, n)) => {
            if size == 0 || size <= 4 {
                return true;
            }
            match core::str::from_utf8(&buf[..n]) {
                Ok(text) => {
                    let trimmed = text.trim_start();
                    trimmed.starts_with("{}") || trimmed.starts_with("[]")
                }
                Err(_) => false,
            }
        }
        Err(_) => true,
    }
}

fn write_dictionary_seed(sd: &SdStorage, path: &str, name: &str, data: &[u8]) -> bool {
    match write_file_in_path(sd, path, name, data) {
        Ok(()) => {
            log::info!("rustmix-provision: repaired dictionary {}/{}", path, name);
            true
        }
        Err(e) => {
            log::warn!(
                "rustmix-provision: repair dictionary {}/{} failed: {:?}",
                path,
                name,
                e
            );
            false
        }
    }
}

fn ensure_path_best_effort(sd: &SdStorage, path: &str) -> bool {
    match ensure_path(sd, path) {
        Ok(()) => true,
        Err(e) => {
            log::warn!("rustmix-provision: ensure {} failed: {:?}", path, e);
            false
        }
    }
}

fn seed_or_repair_sleep_file(sd: &SdStorage, path: &str, name: &str, data: &[u8]) -> bool {
    if !name.ends_with(".BMP") {
        return seed_path_file_if_missing(sd, path, name, data);
    }

    let needs_write = match rustmix_sleep_bmp_is_valid(sd, path, name) {
        Ok(true) => false,
        Ok(false) => true,
        Err(_) => true,
    };

    if !needs_write {
        return true;
    }

    match write_file_in_path(sd, path, name, data) {
        Ok(()) => {
            log::info!("rustmix-provision: seeded/repaired {}/{}", path, name);
            true
        }
        Err(e) => {
            log::warn!(
                "rustmix-provision: seed/repair {}/{} failed: {:?}",
                path,
                name,
                e
            );
            false
        }
    }
}

fn rustmix_sleep_bmp_is_valid(
    sd: &SdStorage,
    path: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<bool> {
    let mut header = [0u8; 64];
    let (size, n) = read_file_start_in_path(sd, path, name, &mut header)?;
    if n < 54 || header.get(0..2) != Some(b"BM") {
        return Ok(false);
    }

    let data_offset = u32::from_le_bytes([header[10], header[11], header[12], header[13]]);
    let dib_size = u32::from_le_bytes([header[14], header[15], header[16], header[17]]);
    let width = i32::from_le_bytes([header[18], header[19], header[20], header[21]]);
    let raw_height = i32::from_le_bytes([header[22], header[23], header[24], header[25]]);
    let planes = u16::from_le_bytes([header[26], header[27]]);
    let bits_per_pixel = u16::from_le_bytes([header[28], header[29]]);
    let compression = u32::from_le_bytes([header[30], header[31], header[32], header[33]]);

    let height = if raw_height < 0 {
        -raw_height
    } else {
        raw_height
    };
    let row_stride = ((800u32 + 31) / 32) * 4;
    let min_size = data_offset.saturating_add(row_stride.saturating_mul(480));

    Ok(dib_size >= 40
        && width == 800
        && height == 480
        && planes == 1
        && bits_per_pixel == 1
        && compression == 0
        && size >= min_size)
}

fn seed_path_file_if_missing(sd: &SdStorage, path: &str, name: &str, data: &[u8]) -> bool {
    match file_size_in_path(sd, path, name) {
        Ok(_) => true,
        Err(_) => match write_file_in_path(sd, path, name, data) {
            Ok(()) => {
                log::info!("rustmix-provision: seeded {}/{}", path, name);
                true
            }
            Err(e) => {
                log::warn!("rustmix-provision: seed {}/{} failed: {:?}", path, name, e);
                false
            }
        },
    }
}

pub fn read_app_data_start(
    sd: &SdStorage,
    name: &str,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<(u32, usize)> {
    read_file_start_in_dir(sd, X4_DIR, name, buf)
        .or_else(|_| read_file_start_in_dir(sd, LEGACY_X4_DIR, name, buf))
}

// async boot path (runs inside the real executor)

pub async fn ensure_x4_dir_async(
    sd: &SdStorage,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    let mut guard = borrow(sd)?;
    let inner = &mut *guard;

    if let Ok(dir) = inner.mgr.open_dir(inner.root, X4_DIR).await {
        let _ = inner.mgr.close_dir(dir);
        return Ok(());
    }
    match inner.mgr.make_dir_in_dir(inner.root, X4_DIR).await {
        Ok(()) => Ok(()),
        Err(embedded_sdmmc::Error::DirAlreadyExists) => Ok(()),
        Err(_) => Err(Error::new(ErrorKind::WriteFailed, "ensure_x4_dir_async")),
    }
}

// /RUSTMIX subdirectory operations

pub fn ensure_x4_subdir(
    sd: &SdStorage,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    let exists = poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, X4_DIR, |x4_h| {
            match inner.mgr.open_dir(x4_h, name).await {
                Ok(sub) => {
                    let _ = inner.mgr.close_dir(sub);
                    Ok::<_, Error>(true)
                }
                Err(_) => Ok(false),
            }
        })
    })?;

    if exists {
        return Ok(());
    }

    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, X4_DIR, |x4_h| {
            match inner.mgr.make_dir_in_dir(x4_h, name).await {
                Ok(()) => Ok::<_, Error>(()),
                Err(embedded_sdmmc::Error::DirAlreadyExists) => Ok(()),
                Err(_) => Err(Error::new(ErrorKind::WriteFailed, "ensure_x4_subdir")),
            }
        })
    })
}

pub fn write_in_x4_subdir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, X4_DIR, dir, |sub_h| op_write!(
            inner, sub_h, name, data
        ))
    })
}

pub fn append_in_x4_subdir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, X4_DIR, dir, |sub_h| op_append!(
            inner, sub_h, name, data
        ))
    })
}

pub fn read_chunk_in_x4_subdir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
    offset: u32,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, X4_DIR, dir, |sub_h| op_read_chunk!(
            inner, sub_h, name, offset, buf
        ))
    })
}

pub fn file_size_in_x4_subdir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<u32> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, X4_DIR, dir, |sub_h| op_file_size!(
            inner, sub_h, name
        ))
    })
}

pub fn delete_in_x4_subdir(
    sd: &SdStorage,
    dir: &str,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_subdir!(inner, X4_DIR, dir, |sub_h| op_delete!(inner, sub_h, name))
    })
}

// /RUSTMIX direct file operations (legacy /_x4 is read-only fallback)

pub fn read_chunk_in_x4(
    sd: &SdStorage,
    name: &str,
    offset: u32,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    let primary = poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, X4_DIR, |dir_h| op_read_chunk!(
            inner, dir_h, name, offset, buf
        ))
    });
    primary.or_else(|_| {
        poll_once(async {
            let mut guard = borrow(sd)?;
            let inner = &mut *guard;
            in_dir!(inner, LEGACY_X4_DIR, |dir_h| op_read_chunk!(
                inner, dir_h, name, offset, buf
            ))
        })
    })
}

pub fn write_in_x4(
    sd: &SdStorage,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, X4_DIR, |dir_h| op_write!(inner, dir_h, name, data))
    })
}

pub fn append_in_x4(
    sd: &SdStorage,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, X4_DIR, |dir_h| op_append!(inner, dir_h, name, data))
    })
}

pub fn file_size_in_x4(
    sd: &SdStorage,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<u32> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, X4_DIR, |dir_h| op_file_size!(inner, dir_h, name))
    })
}

pub fn delete_in_x4(sd: &SdStorage, name: &str) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, X4_DIR, |dir_h| op_delete!(inner, dir_h, name))
    })
}

// seek+write: open existing file, seek to offset, write data, close
// used to update the chapter offset table after all chapters are appended
pub fn write_at_in_x4(
    sd: &SdStorage,
    name: &str,
    offset: u32,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    poll_once(async {
        let mut guard = borrow(sd)?;
        let inner = &mut *guard;
        in_dir!(inner, X4_DIR, |dir_h| {
            match inner
                .mgr
                .open_file_in_dir(dir_h, name, Mode::ReadWriteCreateOrAppend)
                .await
            {
                Err(_) => Err(Error::new(ErrorKind::OpenFile, "write_at")),
                Ok(file) => {
                    let result = match inner.mgr.file_seek_from_start(file, offset) {
                        Ok(()) => inner
                            .mgr
                            .write(file, data)
                            .await
                            .map_err(|_| Error::new(ErrorKind::WriteFailed, "write_at")),
                        Err(_) => Err(Error::new(ErrorKind::SeekFailed, "write_at")),
                    };
                    let _ = inner.mgr.close_file(file).await;
                    result
                }
            }
        })
    })
}

// title mapping

// append a title line to /RUSTMIX/TITLES.BIN
pub fn save_title(
    sd: &SdStorage,
    filename: &str,
    title: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    let name_bytes = filename.as_bytes();
    let title_bytes = title.as_bytes();
    let title_len = title_bytes.len().min(TITLE_CAP);
    let line_len = name_bytes.len() + 1 + title_len + 1; // name + \t + title + \n
    if line_len > 128 {
        return Err(Error::new(
            ErrorKind::WriteFailed,
            "save_title: line too long",
        ));
    }
    let mut line = [0u8; 128];
    line[..name_bytes.len()].copy_from_slice(name_bytes);
    line[name_bytes.len()] = b'\t';
    line[name_bytes.len() + 1..name_bytes.len() + 1 + title_len]
        .copy_from_slice(&title_bytes[..title_len]);
    line[name_bytes.len() + 1 + title_len] = b'\n';

    append_file_in_dir(sd, X4_DIR, TITLES_FILE, &line[..line_len])
}
