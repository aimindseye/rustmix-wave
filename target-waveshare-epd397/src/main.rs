use anyhow::Context;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::sd::{
    mmc::{SdMmcHostConfiguration, SdMmcHostDriver},
    SdCardConfiguration, SdCardDriver,
};
use esp_idf_svc::io::vfs::MountedFatfs;

use esp_idf_hal::{
    gpio::{AnyIOPin, PinDriver, Pull},
    peripherals::Peripherals,
    spi::{config::Config as SpiConfig, Dma, SpiDeviceDriver, SpiDriver, SpiDriverConfig},
    units::Hertz,
};

use hal_waveshare_epd397::{
    board,
    display::{DisplayBackendAdapter, ShellDisplayBridge},
    raw_marker,
    txt_book_browser::render_txt_book_browser_v0,
    reader_foundation::{
        render_reader_page_v0,
        render_reader_page_with_title_v0,
        build_txt_layout_pagination_v0,
        render_reader_layout_page_with_title_v0,
        TxtLayoutPagination,
        ReaderBook,
        ReaderScreenState,
        ReaderStorage,
    },
};

fn main() {
    esp_idf_sys::link_patches();

    raw_marker(b"RAW-RUSTMIX-WAVE-MAIN-ENTER\n\0");

    if let Err(err) = try_main() {
        raw_marker(b"RAW-RUSTMIX-WAVE-BOOT-ERROR\n\0");
        println!("rustmix-wave boot error: {err:?}");

        loop {
            esp_idf_hal::delay::FreeRtos::delay_ms(1000);
        }
    }
}

fn try_main() -> anyhow::Result<()> {
    raw_marker(b"RAW-RUSTMIX-WAVE-TRY-MAIN-ENTER\n\0");

    let peripherals = Peripherals::take().context("peripherals take failed")?;
    let pins = peripherals.pins;
    raw_marker(b"RAW-RUSTMIX-WAVE-PERIPHERALS-OK\n\0");

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-PINMAP-OK\n\0");

    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        pins.gpio11,
        pins.gpio12,
        None::<AnyIOPin>,
        &SpiDriverConfig::new().dma(Dma::Auto(4096)),
    )
    .context("display SPI driver init failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SPI-DRIVER-OK\n\0");

    let spi = SpiDeviceDriver::new(
        spi_driver,
        Some(pins.gpio10),
        &SpiConfig::new().baudrate(Hertz(500_000)),
    )
    .context("display SPI device init failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-SPI-DEVICE-OK\n\0");

    let dc = PinDriver::output(pins.gpio9).context("display DC pin init failed")?;
    let rst = PinDriver::output(pins.gpio46).context("display RST pin init failed")?;
    let busy = PinDriver::input(pins.gpio3, Pull::Floating).context("display BUSY pin init failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-DISPLAY-PINS-OK\n\0");

    // Keep the accepted pin map visible in binary/log-review context.
    println!(
        "rustmix-wave display pins SCLK={} MOSI={} CS={} DC={} RST={} BUSY={}",
        board::EPD_SCLK,
        board::EPD_MOSI,
        board::EPD_CS,
        board::EPD_DC,
        board::EPD_RST,
        board::EPD_BUSY
    );

    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BOOT-FLOW-V0-START\n\0");

    let backend = DisplayBackendAdapter::new(spi, dc, rst, busy);
    let mut shell_display = ShellDisplayBridge::new(backend);

    shell_display
        .init()
        .context("Rustmix-Wave shell display init failed")?;
    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BOOT-DISPLAY-READY-OK\n\0");

    let sd_card_driver = SdCardDriver::new_mmc(
        SdMmcHostDriver::new_4bits(
            peripherals.sdmmc1,
            pins.gpio17,
            pins.gpio16,
            pins.gpio15,
            pins.gpio7,
            pins.gpio8,
            pins.gpio18,
            None::<AnyIOPin>,
            None::<AnyIOPin>,
            &SdMmcHostConfiguration::new(),
        )
        .context("Waveshare SDMMC host init failed")?,
        &SdCardConfiguration::new(),
    )
    .context("Waveshare SD card driver init failed")?;

    let _sd_mount = MountedFatfs::mount(
        Fatfs::new_sdcard(0, sd_card_driver).context("Waveshare SD FATFS init failed")?,
        "/sdcard",
        4,
    )
    .context("Waveshare SD FATFS mount failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BOOT-SD-MOUNT-OK\n\0");

    raw_marker(b"RAW-RUSTMIX-WAVE-BUTTON-NAV-V0-START\n\0");

    let mut button_up = PinDriver::input(pins.gpio4, Pull::Up)
        .context("Waveshare Button_Up GPIO4 input init failed")?;
    let mut button_function = PinDriver::input(pins.gpio5, Pull::Up)
        .context("Waveshare Button_Function GPIO5 input init failed")?;
    let mut button_down = PinDriver::input(pins.gpio6, Pull::Up)
        .context("Waveshare Button_Down GPIO6 input init failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-BUTTON-NAV-PINS-OK\n\0");

    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-V0-START\n\0");

    let browser_books = scan_txt_browser_books_v0()?;
    let mut browser_selected_index = 0usize;
    let mut reader_mode = RustmixWaveReaderMode::Browser;

    render_txt_browser_screen_v0(&mut shell_display, &browser_books, browser_selected_index)
        .context("Rustmix-Wave TXT book browser render failed")?;

    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-BOOT-OK\n\0");

    let mut sd_storage = SdTxtReaderStorage::new();
    let mut sd_state = ReaderScreenState::new_with_total_pages(0, 0, 1);
    let mut reader_title = String::from("TXT BOOK");
    let mut txt_layout = None::<TxtLayoutPagination>;

    raw_marker(b"RAW-RUSTMIX-WAVE-BUTTON-NAV-READY\n\0");
    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BOOT-FLOW-V0-OK\n\0");

    let mut last_up_pressed = button_up.is_low();
    let mut last_function_pressed = button_function.is_low();
    let mut last_down_pressed = button_down.is_low();

    loop {
        let up_pressed = button_up.is_low();
        let function_pressed = button_function.is_low();
        let down_pressed = button_down.is_low();

        if down_pressed && !last_down_pressed {
            match reader_mode {
                RustmixWaveReaderMode::Browser => {
                    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-SELECT-DOWN\n\0");

                    if !browser_books.is_empty() {
                        let max_index = browser_books.len().saturating_sub(1);
                        browser_selected_index =
                            core::cmp::min(browser_selected_index.saturating_add(1), max_index);
                    }

                    render_txt_browser_screen_v0(
                        &mut shell_display,
                        &browser_books,
                        browser_selected_index,
                    )
                    .context("Rustmix-Wave TXT browser select down render failed")?;

                    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-SELECT-DOWN-OK\n\0");
                }
                RustmixWaveReaderMode::Reader => {
                    raw_marker(b"RAW-RUSTMIX-WAVE-BUTTON-DOWN-NEXT\n\0");
                    sd_state.next_page();

                    render_reader_layout_page_with_title_v0(
                        &mut shell_display,
                        reader_title.as_str(),
                        txt_layout
                            .as_ref()
                            .context("Rustmix-Wave TXT layout pagination missing")?,
                        &sd_state,
                    )
                    .context("Rustmix-Wave button next page render failed")?;

                    raw_marker(b"RAW-RUSTMIX-WAVE-BUTTON-DOWN-NEXT-OK\n\0");
                }
            }

            esp_idf_hal::delay::FreeRtos::delay_ms(220);
        }

        if up_pressed && !last_up_pressed {
            match reader_mode {
                RustmixWaveReaderMode::Browser => {
                    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-SELECT-UP\n\0");

                    browser_selected_index = browser_selected_index.saturating_sub(1);

                    render_txt_browser_screen_v0(
                        &mut shell_display,
                        &browser_books,
                        browser_selected_index,
                    )
                    .context("Rustmix-Wave TXT browser select up render failed")?;

                    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-SELECT-UP-OK\n\0");
                }
                RustmixWaveReaderMode::Reader => {
                    raw_marker(b"RAW-RUSTMIX-WAVE-BUTTON-UP-PREV\n\0");
                    sd_state.previous_page();

                    render_reader_layout_page_with_title_v0(
                        &mut shell_display,
                        reader_title.as_str(),
                        txt_layout
                            .as_ref()
                            .context("Rustmix-Wave TXT layout pagination missing")?,
                        &sd_state,
                    )
                    .context("Rustmix-Wave button previous page render failed")?;

                    raw_marker(b"RAW-RUSTMIX-WAVE-BUTTON-UP-PREV-OK\n\0");
                }
            }

            esp_idf_hal::delay::FreeRtos::delay_ms(220);
        }

        if function_pressed && !last_function_pressed {
            match reader_mode {
                RustmixWaveReaderMode::Browser => {
                    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-FUNCTION-OPEN\n\0");

                    let txt_len = open_selected_txt_book_v0(&browser_books, browser_selected_index)?;

                    if txt_len > 0 {
                        if let Some(selected_book) = browser_books.get(browser_selected_index) {
                            reader_title = selected_book.title.clone();
                            raw_marker(b"RAW-RUSTMIX-WAVE-X4-READER-LAYOUT-TITLE-OK\n\0");
                        }

                        txt_layout = Some(
                            build_txt_layout_pagination_v0(&mut sd_storage, 0)
                                .context("Rustmix-Wave TXT layout pagination build failed")?,
                        );

                        let total_pages = txt_layout
                            .as_ref()
                            .map(|layout| layout.total_pages())
                            .unwrap_or(1);

                        sd_state = ReaderScreenState::new_with_total_pages(0, 0, total_pages);

                        render_reader_layout_page_with_title_v0(
                            &mut shell_display,
                            reader_title.as_str(),
                            txt_layout
                                .as_ref()
                                .context("Rustmix-Wave TXT layout pagination missing")?,
                            &sd_state,
                        )
                        .context("Rustmix-Wave selected TXT book first page render failed")?;
                        raw_marker(b"RAW-RUSTMIX-WAVE-X4-TXT-LAYOUT-PAGINATION-TARGET-OK\n\0");

                        reader_mode = RustmixWaveReaderMode::Reader;
                        raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-FUNCTION-OPEN-OK\n\0");
                    }
                }
                RustmixWaveReaderMode::Reader => {
                    raw_marker(b"RAW-RUSTMIX-WAVE-BUTTON-FUNCTION-REFRESH\n\0");

                    render_reader_layout_page_with_title_v0(
                        &mut shell_display,
                        reader_title.as_str(),
                        txt_layout
                            .as_ref()
                            .context("Rustmix-Wave TXT layout pagination missing")?,
                        &sd_state,
                    )
                    .context("Rustmix-Wave button function refresh render failed")?;

                    raw_marker(b"RAW-RUSTMIX-WAVE-BUTTON-FUNCTION-REFRESH-OK\n\0");
                }
            }

            esp_idf_hal::delay::FreeRtos::delay_ms(220);
        }

        last_up_pressed = up_pressed;
        last_function_pressed = function_pressed;
        last_down_pressed = down_pressed;

        esp_idf_hal::delay::FreeRtos::delay_ms(35);
    }
}

// BEGIN RUSTMIX_WAVE_TXT_BOOK_BROWSER_TARGET_V0
struct TxtBrowserBook {
    title: String,
    path: PathBuf,
}

enum RustmixWaveReaderMode {
    Browser,
    Reader,
}

fn txt_browser_title_from_path_v0(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("TXT BOOK")
        .chars()
        .map(|ch| if ch.is_ascii_graphic() || ch == ' ' { ch } else { ' ' })
        .collect()
}

fn scan_txt_browser_books_v0() -> anyhow::Result<Vec<TxtBrowserBook>> {
    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-SCAN-START\n\0");

    let books_dir = Path::new("/sdcard/BOOKS");
    let entries = match fs::read_dir(books_dir) {
        Ok(entries) => entries,
        Err(_) => {
            raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-SCAN-EMPTY\n\0");
            return Ok(Vec::new());
        }
    };

    let mut books = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();

        if !path.is_file() || !is_txt_path_v0(&path) {
            continue;
        }

        books.push(TxtBrowserBook {
            title: txt_browser_title_from_path_v0(&path),
            path,
        });
    }

    books.sort_by(|a, b| a.title.cmp(&b.title));

    if books.is_empty() {
        raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-SCAN-EMPTY\n\0");
    } else {
        raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-SCAN-OK\n\0");
    }

    Ok(books)
}

fn render_txt_browser_screen_v0(
    display: &mut ShellDisplayBridge<'_>,
    books: &[TxtBrowserBook],
    selected_index: usize,
) -> anyhow::Result<()> {
    let titles: Vec<&str> = books.iter().map(|book| book.title.as_str()).collect();
    render_txt_book_browser_v0(display, &titles, selected_index)
}

fn open_selected_txt_book_v0(
    books: &[TxtBrowserBook],
    selected_index: usize,
) -> anyhow::Result<usize> {
    if books.is_empty() {
        raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-OPEN-EMPTY\n\0");
        return Ok(0);
    }

    let selected_index = core::cmp::min(selected_index, books.len().saturating_sub(1));
    let selected = &books[selected_index];

    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-OPEN-SELECTED\n\0");

    fs::create_dir_all(RUSTMIX_WAVE_SD_BOOK_DIR)
        .context("create fixed Rustmix-Wave TXT book dir failed")?;

    fs::copy(&selected.path, RUSTMIX_WAVE_SD_BOOK_PATH)
        .with_context(|| format!("copy selected TXT book failed: {}", selected.path.display()))?;

    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-OPEN-COPIED\n\0");

    let txt_len = fs::metadata(RUSTMIX_WAVE_SD_BOOK_PATH)
        .context("Rustmix-Wave selected TXT metadata failed")?
        .len() as usize;

    raw_marker(b"RAW-RUSTMIX-WAVE-TXT-BROWSER-OPEN-OK\n\0");

    Ok(txt_len)
}
// END RUSTMIX_WAVE_TXT_BOOK_BROWSER_TARGET_V0

// BEGIN RUSTMIX_WAVE_SD_TXT_FIRST_PAGE_V0
const RUSTMIX_WAVE_SD_BOOK_DIR: &str = "/sdcard/RUSTMIX/BOOKS";
const RUSTMIX_WAVE_SD_BOOK_PATH: &str = "/sdcard/RUSTMIX/BOOKS/WAVE.TXT";

static RUSTMIX_WAVE_SD_BOOKS: [ReaderBook; 1] = [ReaderBook {
    id: "WAVETXT",
    title: "SD TXT SAMPLE",
    path: RUSTMIX_WAVE_SD_BOOK_PATH,
}];

const RUSTMIX_WAVE_SD_SAMPLE_TEXT: &str = "\
RUSTMIX WAVE SD TXT FIRST PAGE. THIS TEXT WAS READ FROM THE MICRO SD CARD THROUGH THE READER STORAGE TRAIT. \
THE DISPLAY STILL USES READERDISPLAY SURFACE TO SHELLDISPLAYBRIDGE TO DISPLAYBACKENDADAPTER. \
THIS SLICE PROVES REAL SD TXT READING WITHOUT EPUB WITHOUT BOOKMARK PERSISTENCE WITHOUT PROGRESS WRITES AND WITHOUT REAL ROTARY INPUT. \
THE NEXT STEP CAN REPLACE THIS SAMPLE BOOK WITH A DIRECTORY LISTING AND USER SELECTED TXT FILES.";

struct SdTxtReaderStorage;

impl SdTxtReaderStorage {
    fn new() -> Self {
        Self
    }
}

impl ReaderStorage for SdTxtReaderStorage {
    fn list_books(&mut self) -> anyhow::Result<&'static [ReaderBook]> {
        raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-LIST-OK\n\0");
        Ok(&RUSTMIX_WAVE_SD_BOOKS)
    }

    fn read_file_chunk(
        &mut self,
        path: &str,
        offset: usize,
        buf: &mut [u8],
    ) -> anyhow::Result<usize> {
        let mut file = File::open(path).with_context(|| format!("open SD TXT book failed: {path}"))?;
        file.seek(SeekFrom::Start(offset as u64))
            .with_context(|| format!("seek SD TXT book failed: {path}"))?;
        let n = file
            .read(buf)
            .with_context(|| format!("read SD TXT book failed: {path}"))?;

        raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-READ-OK\n\0");
        Ok(n)
    }

    fn read_state_file(&mut self, _path: &str, _buf: &mut [u8]) -> anyhow::Result<usize> {
        // State persistence is intentionally not enabled in SD TXT First Page v0.
        Ok(0)
    }

    fn write_state_file(&mut self, _path: &str, _data: &[u8]) -> anyhow::Result<()> {
        // Bookmark/progress persistence is intentionally not enabled in this slice.
        Ok(())
    }
}

fn is_txt_path_v0(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("txt"))
        .unwrap_or(false)
}

fn find_first_existing_sd_txt_book_v0() -> anyhow::Result<Option<PathBuf>> {
    // Prefer user-provided books over the generated compatibility target.
    //
    // Spaces in filenames are OK here because std::fs returns PathBuf values and
    // fs::copy receives the full PathBuf. The selected book is copied into the
    // fixed compatibility path used by the current ReaderBook model.
    const USER_SEARCH_DIRS: [&str; 3] = [
        "/sdcard/BOOKS",
        "/sdcard",
        "/sdcard/RUSTMIX/BOOKS",
    ];

    let generated_target = Path::new(RUSTMIX_WAVE_SD_BOOK_PATH);

    for dir in USER_SEARCH_DIRS {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        let mut candidates: Vec<PathBuf> = Vec::new();

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let path = entry.path();

            if !path.is_file() || !is_txt_path_v0(&path) {
                continue;
            }

            // Do not prefer the generated WAVE.TXT when user books exist.
            if path == generated_target {
                continue;
            }

            candidates.push(path);
        }

        candidates.sort();

        if let Some(path) = candidates.into_iter().next() {
            println!("rustmix-wave sd txt user book selected: {}", path.display());
            raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-USER-BOOK-FOUND\n\0");
            return Ok(Some(path));
        }
    }

    // Fallback: use generated compatibility target only when no user TXT exists.
    if generated_target.exists() {
        raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-GENERATED-FALLBACK\n\0");
        return Ok(Some(generated_target.to_path_buf()));
    }

    Ok(None)
}



fn ensure_sd_txt_sample_book_v0() -> anyhow::Result<()> {
    raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-SAMPLE-CHECK-START\n\0");

    fs::create_dir_all(RUSTMIX_WAVE_SD_BOOK_DIR)
        .context("create /sdcard/RUSTMIX/BOOKS failed")?;

    if let Some(source_path) = find_first_existing_sd_txt_book_v0()? {
        let target_path = Path::new(RUSTMIX_WAVE_SD_BOOK_PATH);

        if source_path != target_path {
            fs::copy(&source_path, target_path)
                .with_context(|| format!("copy selected TXT file failed: {}", source_path.display()))?;
            raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-USER-BOOK-COPIED\n\0");
        } else {
            raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-EXISTING-SELECTED\n\0");
        }

        raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-EXISTING-OK\n\0");
        return Ok(());
    }

    fs::write(RUSTMIX_WAVE_SD_BOOK_PATH, RUSTMIX_WAVE_SD_SAMPLE_TEXT.as_bytes())
        .context("write SD TXT sample book failed")?;
    raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-SAMPLE-CREATED\n\0");

    raw_marker(b"RAW-RUSTMIX-WAVE-SD-TXT-SAMPLE-OK\n\0");
    Ok(())
}



// END RUSTMIX_WAVE_SD_TXT_FIRST_PAGE_V0

