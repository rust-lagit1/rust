//! This test checks if unstable feature usage metric dump files `unstable-feature-usage*.json` work
//! as expected.
//!
//! - Basic sanity checks on a default ICE dump.
//!
//! See <https://github.com/rust-lang/rust/issues/129485>.
//!
//! # Test history
//!
//! - forked from dump-ice-to-disk test, which has flakeyness issues on i686-mingw, I'm assuming
//! those will be present in this test as well on the same platform

//@ ignore-windows
//FIXME(#128911): still flakey on i686-mingw.

use std::path::{Path, PathBuf};

use run_make_support::{
    cwd, has_extension, has_prefix, rfs, run_in_tmpdir, rustc, serde_json, shallow_find_files,
};

fn find_feature_usage_metrics<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    shallow_find_files(dir, |path| {
        has_prefix(path, "unstable_feature_usage") && has_extension(path, "json")
    })
}

fn main() {
    test_metrics_dump();
}

#[track_caller]
fn test_metrics_dump() {
    run_in_tmpdir(|| {
        let metrics_dir = cwd().join("metrics");
        rustc()
            .input("lib.rs")
            .env("RUST_BACKTRACE", "short")
            .arg(format!("-Zmetrics-dir={}", metrics_dir.display()))
            .run();
        let mut metrics = find_feature_usage_metrics(&metrics_dir);
        let json_path =
            metrics.pop().expect("there should be exactly metrics file in the output directory");

        assert_eq!(
            0,
            metrics.len(),
            "there should be exactly one metrics file in the output directory"
        );

        let message = rfs::read_to_string(json_path);
        let parsed: serde_json::Value =
            serde_json::from_str(&message).expect("metrics should be dumped as json");
        let expected = serde_json::json!(
            {
                "lib_features":[{"symbol":"ascii_char"}],
                "lang_features":[{"symbol":"box_patterns","since":null}]
            }
        );

        assert_eq!(expected, parsed);
    });
}
