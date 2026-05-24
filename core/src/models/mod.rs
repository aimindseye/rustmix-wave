pub mod book_id;
pub mod book_identity;
pub mod book_identity_title_cache;
pub mod bookmark;
pub mod progress;
pub mod reader_file;
pub mod reader_meta;
pub mod reader_runtime;
pub mod reader_state_io;
pub mod reader_viewport;
pub mod state;
pub mod storage_layout;
pub mod storage_path_helpers;
pub mod theme;

pub use book_id::{BookId, BookIdScheme, FNV1A32_OFFSET, FNV1A32_PRIME, fnv1a32};
pub use book_identity::BookIdentity;
pub use book_identity_title_cache::{
    BookFileFormatModel, BookIdentityTitleModel, HOST_TITLE_MAP_FILE, HOST_TITLE_MAP_PATH,
    StableBookIdModel, TITLE_CACHE_DIR, TITLE_CACHE_FILE, TITLE_CACHE_PATH, TitleCacheRecordModel,
    fallback_display_title_from_filename, is_8dot3_book_id, is_8dot3_cache_key,
    make_title_cache_record, normalize_display_title, normalize_title_cache_key,
    parse_title_cache_record, write_title_cache_record,
};
pub use bookmark::{BookmarkIndexRecord, ReaderBookmark};
pub use progress::ReaderProgress;
pub use reader_file::{
    LIBRARY_DISPLAY_NAME_MAX, LIBRARY_KIND_LABEL_MAX, LIBRARY_PATH_MAX, LibraryEntry,
    LibraryScanPolicy, ReaderFileKind,
};
pub use reader_meta::{BookFormat, ReaderMeta};
pub use reader_runtime::{ReaderNavAction, ReaderPageState, ReaderSessionState, ReaderUiMode};
pub use reader_state_io::{
    BookmarkEntryModel, BookmarkIndexEntryModel, READER_BOOKMARK_EXT, READER_BOOKMARK_INDEX_FILE,
    READER_BOOKMARK_JUMP_PREFIX, READER_PROGRESS_EXT, READER_STATE_DIR, ReaderProgressRecordModel,
    ReaderStateFileFormatModel, bookmark_index_path, bookmark_jump_message,
    bookmark_record_file_for, bookmark_record_path_for, compat_book_id_hex8, parse_bookmark_index,
    parse_bookmark_index_entry, parse_bookmark_jump_message, parse_bookmark_record,
    parse_bookmark_records, parse_progress_record, progress_record_file_for,
    progress_record_path_for, write_bookmark_index, write_bookmark_index_entry,
    write_bookmark_record, write_bookmark_records, write_progress_record,
};
pub use reader_viewport::{
    ReaderViewportCacheKeyFieldsModel, ReaderViewportChromeBasisModel, ReaderViewportModel,
    ReaderViewportPhysicalRotationModel, ReaderViewportRectModel, X4_READER_CONTENT_MARGIN,
    X4_READER_FOOTER_HEIGHT, X4_READER_HEADER_HEIGHT, X4_READER_LANDSCAPE_HEIGHT,
    X4_READER_LANDSCAPE_WIDTH, X4_READER_PORTRAIT_HEIGHT, X4_READER_PORTRAIT_WIDTH,
};
pub use state::{
    ClockFreshnessModel, DisplayPreferencesModel, NetworkTimeStateModel,
    PreparedFallbackPolicyModel, PreparedFontProfileModel, ReaderOrientationModel,
    ReaderPreferencesModel, ReadingThemeModel, SleepImageModeModel, SystemSettingsModel,
    WifiTransferConfigModel, WifiTransferFailureModel, X4_DEFAULT_FCACHE_TARGET, X4_FCACHE_ROOT,
    X4_SETTINGS_COMPAT_PATH, X4_SLEEP_IMAGE_MODE_FILE, X4_TIME_STATE_FILE,
    parse_settings_txt as parse_state_settings_txt, parse_time_txt as parse_state_time_txt,
    write_reader_preferences_txt, write_time_txt as write_state_time_txt,
};
pub use storage_layout::{ReaderStoragePaths, StorageLayout, StorageLayoutKind};
pub use theme::{ReaderThemePreset, ThemeContrast, ThemeKind};
pub mod prepared_cache_metadata;
pub use prepared_cache_metadata::{
    PREPARED_CACHE_ROOT_DIR, PREPARED_CACHE_ROOT_PATH, PREPARED_FONTS_INDEX_FILE,
    PREPARED_META_FILE, PREPARED_PAGES_INDEX_FILE, PreparedCacheBookIdModel,
    PreparedCacheChapterPageModel, PreparedCacheErrorClassModel, PreparedCacheFontIndexModel,
    PreparedCacheKindModel, PreparedCacheManifestModel, PreparedCachePageListModel,
    PreparedCachePageRecordModel, PreparedCacheStatusModel, classify_prepared_cache_error,
    is_valid_prepared_book_id, is_valid_prepared_cache_file, missing_cache_is_safe_fallback,
    parse_prepared_fonts_index, parse_prepared_meta, parse_prepared_page_record,
    parse_prepared_pages_index, prepared_cache_book_path, prepared_cache_file_path,
    prepared_cache_relative_book_path, prepared_cache_relative_file_path,
};
pub mod input_semantic_mapping;
pub mod lua_app_catalog;
pub mod lua_app_discovery;
pub mod lua_app_manifest;
pub mod lua_app_runtime;
pub mod lua_app_storage;
pub mod lua_host_api;
pub use input_semantic_mapping::{
    InputActionKindModel, InputAppContextModel, InputPhysicalButtonModel, InputRepeatPolicyModel,
    InputSemanticActionModel, InputSemanticMappingModel, ReaderPageNavigationModel, context_name,
    date_time_action_for_physical, is_safe_back_action, normalize_physical_button_name,
    reader_action_for_physical, reader_page_navigation_for_action,
    reader_page_navigation_for_physical, repeat_policy_for_context, semantic_action_for_physical,
    wifi_transfer_action_for_physical,
};

pub use lua_app_discovery::{
    LUA_APP_DISCOVERY_DIAGNOSTICS_MAX, LuaAppDiscoveryDiagnosticKindModel,
    LuaAppDiscoveryDiagnosticModel, LuaAppDiscoveryInputModel, LuaAppDiscoveryOutcomeModel,
    discover_lua_apps_from_records, is_safe_lua_app_folder_path, lua_app_absolute_dir,
    lua_app_manifest_path,
};

pub use lua_host_api::{
    LUA_HOST_API_FUNCTION_MAX, LUA_HOST_API_NAMESPACE_MAX, LuaHostApiCallDescriptorModel,
    LuaHostApiErrorModel, LuaHostApiFunctionModel, LuaHostApiNamespaceModel,
    LuaHostApiRuntimeModel, describe_lua_host_api_call, validate_lua_host_api_call,
    validate_lua_host_api_permission, validate_lua_host_api_runtime_call,
};

pub use lua_app_runtime::{
    LUA_APP_RUNTIME_REASON_MAX, LuaAppLifecycleEventModel, LuaAppLifecycleStateModel,
    LuaAppRuntimeDiagnosticModel, LuaAppRuntimeErrorModel, LuaAppRuntimeStateModel,
    LuaAppSafeReturnTargetModel, lua_app_back_event_policy, lua_app_back_exit_diagnostic,
    lua_app_crash_diagnostic, lua_app_exit_diagnostic, lua_app_lifecycle_transition,
    lua_app_safe_return_target_for_state,
};

pub use lua_app_catalog::{
    LUA_APP_CATALOG_DIAGNOSTICS_MAX, LUA_APP_CATALOG_GROUP_MAX, LUA_APP_CATALOG_MAX,
    LUA_APP_CATALOG_SORT_KEY_MAX, LuaAppCatalogCategoryGroupModel,
    LuaAppCatalogDiagnosticKindModel, LuaAppCatalogDiagnosticModel, LuaAppCatalogEntryModel,
    LuaAppCatalogSourceModel, LuaAppCatalogVisibilityModel, LuaAppDashboardCatalogModel,
    LuaDashboardCategoryModel, lua_app_catalog_sort_key, lua_app_catalog_visibility_for_state,
    native_apps_are_authoritative,
};

pub use lua_app_storage::{
    LUA_APP_CACHE_ROOT, LUA_APP_DATA_ROOT, LUA_APP_STATE_ROOT, LUA_APP_STORAGE_ABSOLUTE_PATH_MAX,
    LUA_APP_STORAGE_RELATIVE_PATH_MAX, LUA_APP_STORAGE_ROOT, LuaAppStorageAccessModel,
    LuaAppStorageErrorModel, LuaAppStorageOperationModel, LuaAppStoragePathModel,
    LuaAppStorageRequestModel, LuaAppStorageRootModel, is_safe_lua_app_storage_relative_path,
    lua_app_storage_absolute_path, lua_app_storage_root_path, validate_lua_app_storage_operation,
    validate_lua_app_storage_root_access,
};

pub use lua_app_manifest::{
    LUA_APP_CAPABILITIES_MAX, LUA_APP_ENTRY_MAX, LUA_APP_ID_MAX, LUA_APP_MANIFEST_FILE,
    LUA_APP_NAME_MAX, LUA_APP_PATH_MAX, LUA_APP_REGISTRY_MAX, LUA_APP_VERSION_MAX, LUA_APPS_ROOT,
    LuaAppCapabilityModel, LuaAppCategoryModel, LuaAppManifestErrorModel, LuaAppManifestModel,
    LuaAppRegistryModel, LuaAppTypeModel, LuaDiscoveredAppModel, is_safe_lua_entry_path,
    is_valid_lua_app_id, is_valid_lua_app_version, parse_lua_app_manifest,
};

pub use storage_path_helpers::{
    BOOKMARK_INDEX_FILE, BOOKS_LIBRARY_ROOT, CURRENT_LIBRARY_ROOT, FCACHE_ROOT,
    ReaderFileExtensionModel, SETTINGS_PATH, SLEEP_DAILY_ROOT, SLEEP_MODE_PATH, SLEEP_ROOT,
    STATE_DIR, STATE_ROOT, StoragePathClassModel, StoragePathModel, StorageRootModel, X4_ROOT,
    classify_storage_path, fcache_book_path, fcache_root_path, file_name_from_path,
    has_path_traversal, is_8dot3_file_name, is_safe_path_segment, library_path_books,
    library_path_current, normalize_book_id, safe_join, settings_path, sleep_daily_image_path,
    sleep_daily_root_path, sleep_mode_path, sleep_root_path, state_bookmark_path,
    state_progress_path, title_cache_path,
};
pub mod display_drawing_abstractions;
pub mod wifi_transfer_config;
pub use display_drawing_abstractions::{
    DisplayAppContextLayoutModel, DisplayChromeLayoutModel, DisplayChromeRoleModel,
    DisplayDiagnosticPlacementModel, DisplayOrientationModel, DisplayPointModel,
    DisplayRegionModel, DisplaySizeModel, X4_BODY_HEIGHT, X4_BODY_Y, X4_FOOTER_HEIGHT, X4_FOOTER_Y,
    X4_HEADER_HEIGHT, X4_SCREEN_HEIGHT, X4_SCREEN_WIDTH, cache_diagnostic_placement,
    layout_for_context, reader_cache_diagnostic_region, safe_clip_region, x4_screen_size,
};

pub mod sd_font_catalog;

pub use sd_font_catalog::{
    SD_FONT_CRC_HEX_LEN, SD_FONT_DISPLAY_NAME_MAX, SD_FONT_FILE_EXTENSION, SD_FONT_ID_MAX,
    SD_FONT_MANIFEST_FILE, SD_FONT_MANIFEST_PATH, SD_FONT_MAX_FILE_BYTES, SD_FONT_MAX_GLYPHS,
    SD_FONT_PATH_MAX, SD_FONT_ROOT_DIR, SD_FONT_ROOT_PATH, SdFontCatalogErrorModel,
    SdFontCatalogRecordModel, SdFontScriptModel, SdFontStyleModel, is_crc32_hex,
    is_safe_sd_font_file_name, is_safe_sd_font_id, parse_sd_font_manifest_line,
};
