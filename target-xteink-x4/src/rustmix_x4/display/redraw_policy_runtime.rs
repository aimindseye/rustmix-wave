//! Rustmix-owned redraw policy for the imported X4 app layer.
//!
//! The physical SSD1677 refresh path remains in the existing runtime. This
//! wrapper only decides which app-layer redraw requests should become visible
//! on e-paper. EPUB loading and background caching can enqueue several
//! loading-only redraws before readable page content is ready; those transient
//! states are useful for LCDs, but they create visible flashing on the X4 panel.

use crate::rustmix_x4::apps::manager::AppManager;
use crate::rustmix_x4::x4_apps::apps::{AppContext, AppId, Redraw, Transition};
use crate::rustmix_x4::x4_apps::ui::Region;
use crate::rustmix_x4::x4_kernel::board::Epd;
use crate::rustmix_x4::x4_kernel::drivers::input::Event;
use crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage;
use crate::rustmix_x4::x4_kernel::drivers::ssd1677::Rotation;
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
use crate::rustmix_x4::x4_kernel::kernel::BookmarkCache;
use crate::rustmix_x4::x4_kernel::kernel::KernelHandle;
use crate::rustmix_x4::x4_kernel::kernel::app::AppLayer;
use crate::rustmix_x4::x4_kernel::kernel::config::{SystemSettings, WifiConfig};
use crate::rustmix_x4::x4_kernel::kernel::rtc_session::RtcSession;
use esp_hal::delay::Delay;
use log::info;

const EPUB_FULL_REDRAW_HYDRATION_STEPS: usize = 4;
const LOADING_MSG_PREFIX: &str = "Loading";
const LOADING_PAGE_MSG_PREFIX: &str = "Loading page";
const CACHE_MSG_PREFIX: &str = "Cache";
const INDEXING_MSG_PREFIX: &str = "Indexing";

/// App-layer wrapper used by the Rustmix X4 target.
///
/// The wrapper does not send SSD1677 commands, does not change SPI arbitration,
/// and does not draw pixels itself. It coalesces app-layer redraw requests before
/// the imported scheduler asks the display backend to refresh.
pub struct RustmixRedrawPolicyAppLayer {
    inner: AppManager,
    suppressed_reader_loading_region: Option<Region>,
    suppressed_transient_reader_redraws: u16,
}

impl RustmixRedrawPolicyAppLayer {
    pub const MARKER: &'static str = "rustmix-epub-redraw-policy-v2-ok";

    pub fn new(inner: AppManager) -> Self {
        info!("{}", Self::MARKER);
        Self {
            inner,
            suppressed_reader_loading_region: None,
            suppressed_transient_reader_redraws: 0,
        }
    }

    #[inline]
    pub fn app_shell(&self) -> &crate::rustmix_x4::x4_kernel::app::AppShell {
        self.inner.app_shell()
    }

    #[inline]
    fn active_reader_epub(&self) -> bool {
        self.inner.active() == AppId::Reader && self.inner.reader.is_epub()
    }

    #[inline]
    fn loading_msg_is_cache(msg: &str) -> bool {
        msg.starts_with(CACHE_MSG_PREFIX)
    }

    #[inline]
    fn loading_msg_is_final_page_load(msg: &str) -> bool {
        msg.starts_with(LOADING_PAGE_MSG_PREFIX)
    }

    #[inline]
    fn loading_msg_is_transient_epub(msg: &str) -> bool {
        msg.starts_with(LOADING_MSG_PREFIX) || msg.starts_with(INDEXING_MSG_PREFIX)
    }

    #[inline]
    fn transient_reader_loading_active(&self) -> bool {
        self.active_reader_epub()
            && self.inner.ctx().loading_active()
            && Self::loading_msg_is_transient_epub(self.inner.ctx().loading_msg())
    }

    fn remember_suppressed_loading_region(&mut self) {
        if self.inner.ctx().loading_active() {
            self.suppressed_reader_loading_region = Some(self.inner.ctx().loading_region());
        }
    }

    async fn hydrate_reader_before_full_redraw(&mut self, k: &mut KernelHandle<'_>) {
        if !self.active_reader_epub() {
            return;
        }

        // Reader advances EPUB open through transient states: Loading 10/25/40,
        // Caching 55, Indexing 75, Loading page 90, then Ready. A full transition
        // frame should show either the first readable page or the previous screen
        // while the page is prepared; it should not show every loading percentage.
        let mut previous_was_final_page_load =
            Self::loading_msg_is_final_page_load(self.inner.ctx().loading_msg());

        for _ in 0..EPUB_FULL_REDRAW_HYDRATION_STEPS {
            if !self.active_reader_epub() || !self.inner.ctx().loading_active() {
                break;
            }

            self.inner.run_background(k).await;

            let loading_active = self.inner.ctx().loading_active();
            let loading_msg = self.inner.ctx().loading_msg();
            let cache_loading = Self::loading_msg_is_cache(loading_msg);

            if !loading_active {
                self.suppressed_transient_reader_redraws = 0;
                break;
            }

            if previous_was_final_page_load && cache_loading && self.inner.reader.has_bg_work() {
                // Page content is ready; remaining text is background EPUB cache
                // progress. Keep caching, but do not flash cache progress over the
                // readable page.
                break;
            }

            previous_was_final_page_load = Self::loading_msg_is_final_page_load(loading_msg);
        }
    }

    fn suppress_loading_region_redraw(&mut self, redraw: Redraw) -> Redraw {
        let Some(region) = self.suppressed_reader_loading_region.take() else {
            return redraw;
        };

        match redraw {
            Redraw::Partial(r) if r == region => {
                info!(
                    "display: dropped reader loading-only partial redraw at {:?}",
                    r
                );
                Redraw::None
            }
            other => other,
        }
    }

    fn suppress_transient_reader_loading_redraw(&mut self, redraw: Redraw) -> Redraw {
        if !self.transient_reader_loading_active() {
            self.suppressed_transient_reader_redraws = 0;
            return self.suppress_loading_region_redraw(redraw);
        }

        match redraw {
            Redraw::None => Redraw::None,
            Redraw::Partial(r) => {
                self.suppressed_transient_reader_redraws =
                    self.suppressed_transient_reader_redraws.saturating_add(1);
                self.remember_suppressed_loading_region();
                info!(
                    "display: deferred transient EPUB loading partial redraw at {:?}: {} {}%",
                    r,
                    self.inner.ctx().loading_msg(),
                    self.inner.ctx().loading_pct()
                );
                Redraw::None
            }
            Redraw::Full => {
                // Keep full transition redraws so the scheduler can call
                // pre_render_full_redraw(), which hydrates the reader before
                // the frame is written. Suppressing Full here would skip that
                // hook and could leave the previous app chrome on-screen.
                Redraw::Full
            }
        }
    }

    pub const fn owns_physical_refresh() -> bool {
        false
    }

    pub const fn owns_spi_arbitration() -> bool {
        false
    }
}

impl AppLayer for RustmixRedrawPolicyAppLayer {
    type Id = AppId;

    #[inline]
    fn active(&self) -> AppId {
        self.inner.active()
    }

    fn dispatch_event(&mut self, event: Event, bm: &mut BookmarkCache) -> Transition {
        self.inner.dispatch_event(event, bm)
    }

    fn apply_transition(&mut self, t: Transition, k: &mut KernelHandle<'_>) {
        self.inner.apply_transition(t, k);
    }

    async fn run_background(&mut self, k: &mut KernelHandle<'_>) {
        self.inner.run_background(k).await;
    }

    async fn pre_render_full_redraw(&mut self, k: &mut KernelHandle<'_>) {
        if self.active_reader_epub() {
            self.hydrate_reader_before_full_redraw(k).await;
        } else {
            self.inner.pre_render_full_redraw(k).await;
        }
    }

    fn draw(&self, strip: &mut StripBuffer) {
        self.inner.draw(strip);
    }

    #[inline]
    fn desired_display_rotation(&self) -> Rotation {
        self.inner.desired_display_rotation()
    }

    #[inline]
    fn has_redraw(&self) -> bool {
        self.inner.has_redraw()
    }

    #[inline]
    fn take_redraw(&mut self) -> Redraw {
        let redraw = self.inner.take_redraw();
        self.suppress_transient_reader_loading_redraw(redraw)
    }

    #[inline]
    fn request_full_redraw(&mut self) {
        self.inner.request_full_redraw();
    }

    #[inline]
    fn ctx_mut(&mut self) -> &mut AppContext {
        self.inner.ctx_mut()
    }

    fn system_settings(&self) -> &SystemSettings {
        self.inner.system_settings()
    }

    fn settings_loaded(&self) -> bool {
        self.inner.settings_loaded()
    }

    fn ghost_clear_every(&self) -> u32 {
        self.inner.ghost_clear_every()
    }

    fn wifi_config(&self) -> &WifiConfig {
        self.inner.wifi_config()
    }

    fn load_eager_settings(&mut self, k: &mut KernelHandle<'_>) {
        <AppManager as AppLayer>::load_eager_settings(&mut self.inner, k);
    }

    fn load_initial_state(&mut self, k: &mut KernelHandle<'_>) {
        <AppManager as AppLayer>::load_initial_state(&mut self.inner, k);
    }

    fn enter_initial(&mut self, k: &mut KernelHandle<'_>) {
        <AppManager as AppLayer>::enter_initial(&mut self.inner, k);
    }

    fn collect_session(&self, session: &mut RtcSession) {
        <AppManager as AppLayer>::collect_session(&self.inner, session);
    }

    fn apply_session(&mut self, session: &RtcSession, k: &mut KernelHandle<'_>) -> bool {
        <AppManager as AppLayer>::apply_session(&mut self.inner, session, k)
    }

    fn needs_special_mode(&self) -> bool {
        <AppManager as AppLayer>::needs_special_mode(&self.inner)
    }

    async fn run_special_mode(
        &mut self,
        epd: &mut Epd,
        strip: &mut StripBuffer,
        delay: &mut Delay,
        sd: &SdStorage,
    ) {
        <AppManager as AppLayer>::run_special_mode(&mut self.inner, epd, strip, delay, sd).await;
    }

    fn suppress_deferred_input(&self) -> bool {
        <AppManager as AppLayer>::suppress_deferred_input(&self.inner)
    }
}
