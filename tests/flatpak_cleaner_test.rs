use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use tempfile::tempdir;
use mithrandir_rs::{
    clean_flatpak_cache, clean_flatpak_cache_with_running, is_protected_path,
    scan_flatpak_cache,
};

#[test]
fn test_flatpak_safe_cleaning_preserves_zen_and_profiles() {
    let temp = tempdir().expect("Failed to create temporary directory");
    let var_app = temp.path();

    // Mock Zen Browser structure:
    // ~/.var/app/app.zen_browser.zen/.zen/Default (release)/prefs.js
    // ~/.var/app/app.zen_browser.zen/.zen/Default (release)/zen-sessions.jsonlz4
    // ~/.var/app/app.zen_browser.zen/.zen/Default (release)/places.sqlite
    // ~/.var/app/app.zen_browser.zen/cache/zen/Default (release)/cache2/entries/dummy_cache.bin
    let zen_app_dir = var_app.join("app.zen_browser.zen");
    let zen_profile_dir = zen_app_dir.join(".zen/Default (release)");
    fs::create_dir_all(&zen_profile_dir).expect("Failed to create mock .zen directory");

    let prefs_path = zen_profile_dir.join("prefs.js");
    fs::write(&prefs_path, "user_pref('browser.startup.homepage', 'about:blank');").unwrap();

    let sessions_path = zen_profile_dir.join("zen-sessions.jsonlz4");
    fs::write(&sessions_path, b"mozLz40\0mock_session_stream").unwrap();

    let places_path = zen_profile_dir.join("places.sqlite");
    fs::write(&places_path, b"SQLite format 3\0mock_history_database").unwrap();

    // Cache structure
    let cache_dir = zen_app_dir.join("cache");
    let cache_entries_dir = cache_dir.join("zen/Default (release)/cache2/entries");
    fs::create_dir_all(&cache_entries_dir).expect("Failed to create mock cache entries directory");

    let dummy_cache_file = cache_entries_dir.join("dummy_cache.bin");
    fs::write(&dummy_cache_file, b"volatile_network_cache_block_12345").unwrap();

    // Symlink inside cache/ pointing outside to .zen/.../places.sqlite
    let symlink_path = cache_dir.join("symlink_to_places");
    symlink("../.zen/Default (release)/places.sqlite", &symlink_path).unwrap();

    // Internal protected file misplaced inside cache/
    let misplaced_db = cache_dir.join("cookies.sqlite");
    fs::write(&misplaced_db, b"SQLite format 3\0mock_cookies").unwrap();

    // Protected backup directory inside cache/
    let backups_dir = cache_dir.join("bookmarkbackups");
    fs::create_dir_all(&backups_dir).unwrap();
    let backup_file = backups_dir.join("bookmarks.jsonlz4");
    fs::write(&backup_file, b"mozLz40\0mock_bookmarks").unwrap();

    // Scan should only count un-protected files (dummy_cache.bin)
    let (scanned_bytes, scanned_count) = scan_flatpak_cache(var_app);
    assert_eq!(scanned_count, 1, "Only dummy_cache.bin should be marked for deletion");
    assert_eq!(scanned_bytes, b"volatile_network_cache_block_12345".len() as u64);

    // Run clean
    let (freed_bytes, freed_count) = clean_flatpak_cache(var_app);
    assert_eq!(freed_count, 1);
    assert_eq!(freed_bytes, b"volatile_network_cache_block_12345".len() as u64);

    // Assertions:
    // 1. dummy_cache.bin MUST be deleted
    assert!(!dummy_cache_file.exists(), "dummy_cache.bin MUST be deleted by cleaner");

    // 2. Profile files MUST remain intact and untouched
    assert!(prefs_path.exists(), "prefs.js MUST remain intact");
    assert_eq!(
        fs::read_to_string(&prefs_path).unwrap(),
        "user_pref('browser.startup.homepage', 'about:blank');"
    );

    assert!(sessions_path.exists(), "zen-sessions.jsonlz4 MUST remain intact");
    assert_eq!(fs::read(&sessions_path).unwrap(), b"mozLz40\0mock_session_stream");

    assert!(places_path.exists(), "places.sqlite MUST remain intact");
    assert_eq!(fs::read(&places_path).unwrap(), b"SQLite format 3\0mock_history_database");

    // 3. Symlink pointing outside MUST be preserved
    assert!(symlink_path.is_symlink(), "Symlink pointing outside cache MUST be preserved");

    // 4. Misplaced protected files & directories inside cache MUST be preserved
    assert!(misplaced_db.exists(), "cookies.sqlite in cache must be preserved by signature filter");
    assert!(backup_file.exists(), "bookmarkbackups in cache must be preserved by directory filter");
}

#[test]
fn test_flatpak_skip_running_application() {
    let temp = tempdir().expect("Failed to create temporary directory");
    let var_app = temp.path();

    let firefox_dir = var_app.join("org.mozilla.firefox");
    let cache_dir = firefox_dir.join("cache/entries");
    fs::create_dir_all(&cache_dir).unwrap();
    let cache_file = cache_dir.join("active_session_cache.tmp");
    fs::write(&cache_file, b"active_firefox_session_data").unwrap();

    let mut running = HashSet::new();
    running.insert("org.mozilla.firefox".to_string());

    let (freed_bytes, freed_count) = clean_flatpak_cache_with_running(var_app, &running);
    assert_eq!(freed_count, 0, "Running applications must be skipped");
    assert_eq!(freed_bytes, 0);
    assert!(cache_file.exists(), "Cache of running application must not be deleted");
}

#[test]
fn test_is_protected_path_predicates() {
    let cache_root = Path::new("/home/user/.var/app/app.zen_browser.zen/cache");

    // Root is protected
    assert!(is_protected_path(cache_root, cache_root));

    // Protected filenames
    assert!(is_protected_path(&cache_root.join("prefs.js"), cache_root));
    assert!(is_protected_path(&cache_root.join("user.js"), cache_root));
    assert!(is_protected_path(&cache_root.join("logins.json"), cache_root));
    assert!(is_protected_path(&cache_root.join("lock"), cache_root));

    // Protected extensions
    assert!(is_protected_path(&cache_root.join("history.sqlite"), cache_root));
    assert!(is_protected_path(&cache_root.join("history.sqlite-wal"), cache_root));
    assert!(is_protected_path(&cache_root.join("session.jsonlz4"), cache_root));
    assert!(is_protected_path(&cache_root.join("config.ini"), cache_root));
    assert!(is_protected_path(&cache_root.join("profiles.ini"), cache_root));

    // Protected directories
    assert!(is_protected_path(&cache_root.join("bookmarkbackups/data.bin"), cache_root));
    assert!(is_protected_path(&cache_root.join("zen-sessions-backup/session.bak"), cache_root));
    assert!(is_protected_path(&cache_root.join("extensions/my-addon"), cache_root));
    assert!(is_protected_path(&cache_root.join("chrome/userChrome.css"), cache_root));

    // Non-protected cache files
    assert!(!is_protected_path(&cache_root.join("cache2/entries/001A2B"), cache_root));
    assert!(!is_protected_path(&cache_root.join("thumbnails/image.jpg"), cache_root));
    assert!(!is_protected_path(&cache_root.join("shader_cache.bin"), cache_root));
}
