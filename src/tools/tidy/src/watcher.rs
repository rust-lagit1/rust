//! Checks that text between tags unchanged, emitting warning otherwise,
//! allowing asserting that code in different places over codebase is in sync.
//!
//! This works via hashing text between tags and saving hash in tidy.
//!
//! Usage:
//!
//! some.rs:
//! // tidy-ticket-foo
//! const FOO: usize = 42;
//! // tidy-ticket-foo
//!
//! some.sh:
//! # tidy-ticket-foo
//! export FOO=42
//! # tidy-ticket-foo
use md5::{Digest, Md5};
use std::fs;
use std::path::Path;

#[cfg(test)]
mod tests;

/// Return hash for source text between 2 tag occurrence,
/// ignoring lines where tag written
///
/// Expecting:
/// tag is not multiline
/// source always have at least 2 occurrence of tag (>2 ignored)
fn span_hash(source: &str, tag: &str, bad: &mut bool) -> Result<String, ()> {
    let start_idx = match source.find(tag) {
        Some(idx) => idx,
        None => return Err(tidy_error!(bad, "tag {} should exist in provided text", tag)),
    };
    let end_idx = {
        let end = match source[start_idx + tag.len()..].find(tag) {
            // index from source start
            Some(idx) => start_idx + tag.len() + idx,
            None => return Err(tidy_error!(bad, "tag end {} should exist in provided text", tag)),
        };
        // second line with tag can contain some other text before tag, ignore it
        // by finding position of previous line ending
        //
        // FIXME: what if line ending is \r\n? In that case \r will be hashed too
        let offset = source[start_idx..end].rfind('\n').unwrap();
        start_idx + offset
    };

    let mut hasher = Md5::new();

    source[start_idx..end_idx]
        .lines()
        // skip first line with tag
        .skip(1)
        // hash next lines, ignoring end trailing whitespaces
        .for_each(|line| {
            let trimmed = line.trim_end();
            hasher.update(trimmed);
        });
    Ok(format!("{:x}", hasher.finalize()))
}

fn check_entry(entry: &ListEntry<'_>, bad: &mut bool, root_path: &Path) {
    let file = fs::read_to_string(root_path.join(Path::new(entry.0)))
        .unwrap_or_else(|e| panic!("{:?}, path: {}", e, entry.0));
    let actual_hash = span_hash(&file, entry.2, bad).unwrap();
    if actual_hash != entry.1 {
        // Write tidy error description for wather only once.
        // Will not work if there was previous errors of other types.
        if *bad == false {
            tidy_error!(
                bad,
                "Mismatched hashes for tidy watcher found.\n\
                Check src/tools/tidy/src/watcher.rs, find tag/hash in TIDY_WATCH_LIST list \
                and verify that sources for provided group of tags in sync. If they in sync, update hash."
            )
        }
        tidy_error!(
            bad,
            "hash for tag `{}` in path `{}` mismatch:\n  actual: `{}`, expected: `{}`\n",
            entry.2,
            entry.0,
            actual_hash,
            entry.1
        );
    }
}

/// (path, hash, tag)
type ListEntry<'a> = (&'a str, &'a str, &'a str);

/// List of tags to watch, along with paths and hashes
#[rustfmt::skip]
const TIDY_WATCH_LIST: &[ListEntry<'_>] = &[
    // sync perf commit across dockerfile and opt-dist
    ("src/tools/opt-dist/src/main.rs", "728c2783154a52a30bdb1d66f8ea1f2a", "tidy-ticket-perf-commit"),
    ("src/ci/docker/host-x86_64/dist-x86_64-linux/Dockerfile", "76c8d9783e38e25a461355f82fcd7955", "tidy-ticket-perf-commit"),

    ("compiler/rustc_ast/src/token.rs", "70666de80ab0194a67524deeda3c01b8", "tidy-ticket-ast-from_token"),
    ("compiler/rustc_ast/src/token.rs", "9a78008a2377486eadf19d67ee4fdce2", "tidy-ticket-ast-can_begin_literal_maybe_minus"),
    ("compiler/rustc_parse/src/parser/expr.rs", "500240cdc80690209060fdce10ce065a", "tidy-ticket-rustc_parse-can_begin_literal_maybe_minus"),

    ("compiler/rustc_builtin_macros/src/assert/context.rs", "81bd6f37797c22fce5c85a4b590b3856", "tidy-ticket-all-expr-kinds"),
    ("tests/ui/macros/rfc-2011-nicer-assert-messages/all-expr-kinds.rs", "78ce54cc25baeac3ae07c876db25180c", "tidy-ticket-all-expr-kinds"),

    ("compiler/rustc_const_eval/src/interpret/validity.rs", "91c69e391741f64b7624e1bda4b31bc3", "tidy-ticket-try_visit_primitive"),
    ("compiler/rustc_const_eval/src/interpret/validity.rs", "05e496c9ca019273c49ba9de48b5da23", "tidy-ticket-visit_value"),

    // sync self-profile-events help mesage with actual list of events
    ("compiler/rustc_data_structures/src/profiling.rs", "881e7899c7d6904af1bc000594ee0418", "tidy-ticket-self-profile-events"),
    ("compiler/rustc_session/src/options.rs", "012ee5a3b61ee1377744e5c6913fa00a", "tidy-ticket-self-profile-events"),

    ("compiler/rustc_errors/src/json.rs", "5907da5c0476785fe2aae4d0d62f7171", "tidy-ticket-UnusedExterns"),
    ("src/librustdoc/doctest.rs", "b5bb5128abb4a2dbb47bb1a1a083ba9b", "tidy-ticket-UnusedExterns"),

    ("compiler/rustc_middle/src/ty/util.rs", "cae64b1bc854e7ee81894212facb5bfa", "tidy-ticket-static_ptr_ty"),
    ("compiler/rustc_middle/src/ty/util.rs", "6f5ead08474b4d3e358db5d3c7aef970", "tidy-ticket-thread_local_ptr_ty"),

    ("compiler/rustc_mir_build/src/thir/pattern/deconstruct_pat.rs", "8ac64f1266a60bb7b11d80ac764e5154", "tidy-ticket-arity"),
    ("compiler/rustc_mir_build/src/thir/pattern/deconstruct_pat.rs", "2bab79a2441e8ffae79b7dc3befe91cf", "tidy-ticket-wildcards"),

    ("compiler/rustc_mir_build/src/thir/pattern/deconstruct_pat.rs", "3844ca4b7b45be1c721c17808ee5b2e2", "tidy-ticket-is_covered_by"),
    ("compiler/rustc_mir_build/src/thir/pattern/deconstruct_pat.rs", "4d296b7b1f48a9dd92e8bb8cd3344718", "tidy-ticket-is_covered_by_any"),

    ("compiler/rustc_monomorphize/src/partitioning.rs", "f4f33e9c14f4e0c3a20b5240ae36a7c8", "tidy-ticket-short_description"),
    ("compiler/rustc_codegen_ssa/src/back/write.rs", "5286f7f76fcf564c98d7a8eaeec39b18", "tidy-ticket-short_description"),

    ("compiler/rustc_session/src/config/sigpipe.rs", "8d765a5c613d931852c0f59ed1997dcd", "tidy-ticket-sigpipe"),
    ("library/std/src/sys/unix/mod.rs", "2cdc37081831cdcf44f3331efbe440af", "tidy-ticket-sigpipe"),

    ("compiler/rustc_trait_selection/src/solve/assembly/structural_traits.rs", "b205939890472130649d5fd4fc86a992", "tidy-ticket-extract_tupled_inputs_and_output_from_callable"),
    ("compiler/rustc_trait_selection/src/traits/select/candidate_assembly.rs", "e9a77bba86a02702af65b2713af47394", "tidy-ticket-assemble_fn_pointer_candidates"),

    ("compiler/rustc_trait_selection/src/solve/eval_ctxt/select.rs", "d0c807d90501f3f63dffc3e7ec046c20", "tidy-ticket-rematch_unsize"),
    ("compiler/rustc_trait_selection/src/solve/trait_goals.rs", "f1b0ce28128b5d5a5b545af3f3cf55f4", "tidy-ticket-consider_builtin_unsize_candidate"),

    ("compiler/rustc_trait_selection/src/traits/project.rs", "66585f93352fe56a5be6cc5a63bcc756", "tidy-ticket-assemble_candidates_from_impls-UserDefined"),
    ("compiler/rustc_ty_utils/src/instance.rs", "e8b404fd4160512708f922140a8bb187", "tidy-ticket-resolve_associated_item-UserDefined"),

    ("compiler/rustc_hir_analysis/src/lib.rs", "842e23fb65caf3a96681686131093316", "tidy-ticket-sess-time-item_types_checking"),
    ("src/librustdoc/core.rs", "d11d64105aa952bbf3c0c2f211135c43", "tidy-ticket-sess-time-item_types_checking"),

    ("library/core/src/ptr/metadata.rs", "57fc0e05c177c042c9766cc1134ae240", "tidy-ticket-static_assert_expected_bounds_for_metadata"),
    ("library/core/tests/ptr.rs", "13ecb32e2a0db0998ff94f33a30f5cfd", "tidy-ticket-static_assert_expected_bounds_for_metadata"),
];

pub fn check(root_path: &Path, bad: &mut bool) {
    for entry in TIDY_WATCH_LIST {
        check_entry(entry, bad, root_path);
    }
}
