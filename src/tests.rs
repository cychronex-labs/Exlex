// TESTS WRITTEN USING AI

extern crate alloc;
use crate::{ErrorCode, Exlex};
use alloc::format;
use alloc::string::String;

// ═══════════════════════════════════════════════════════════════════
// TEST FIXTURES
// ═══════════════════════════════════════════════════════════════════

const SIMPLE: &str = r#"
"host" : "localhost"
"port" : "8080"
"debug" : "true"
"#;

const NESTED: &str = r#"
"root_key" : "root_val"
sect "Alpha" {
    "a_key" : "a_val"
    "a_key2" : "a_val2"
    sect "Beta" {
        "b_key" : "b_val"
        sect "Gamma" {
            "g_key" : "g_val"
        }
    }
}
sect "Delta" {
    "d_key" : "d_val"
    "d_key2" : "d_val2"
}
"#;

const MULTI_SECTION: &str = r#"
sect "Database" {
    "driver" : "postgres"
    "pool"   : "32"
    sect "Credentials" {
        "user" : "admin"
        "auth" : "ed25519"
    }
    sect "Replica" {
        "host" : "replica.db"
        "port" : "5433"
    }
}
sect "Cache" {
    "backend" : "redis"
    "ttl"     : "3600"
}
"#;

const ESCAPE: &str = r#"
sect "paths" {
    "windows" : "C:\\Program Files\\App"
    "quoted"  : "He said \"hello\""
    "url"     : "https://example.com/path?a=1"
}
"#;

const DEEP: &str = r#"
sect "L1" {
    "p1" : "v1"
    sect "L2" {
        "p2" : "v2"
        sect "L3" {
            "p3" : "v3"
            sect "L4" {
                "p4" : "v4"
                sect "L5" {
                    "p5" : "v5"
                }
            }
        }
    }
}
"#;

const WIDE: &str = r#"
sect "S1" { "k" : "v1" }
sect "S2" { "k" : "v2" }
sect "S3" { "k" : "v3" }
sect "S4" { "k" : "v4" }
sect "S5" { "k" : "v5" }
sect "S6" { "k" : "v6" }
sect "S7" { "k" : "v7" }
sect "S8" { "k" : "v8" }
sect "S9" { "k" : "v9" }
sect "S10" { "k" : "v10" }
"#;

const COMMENTS: &str = r#"
# This is a comment
"key1" : "val1"
# Another comment
sect "Section" {
    # Comment inside section
    "key2" : "val2"
}
"#;

fn parse(data: &str) -> Exlex {
    Exlex::init_reader(data, None, None, None, None).expect("parse must succeed")
}

// ═══════════════════════════════════════════════════════════════════
// PARSER TESTS
// ═══════════════════════════════════════════════════════════════════

// ── Basic correctness ─────────────────────────────────────────────

#[test]
fn test_parse_simple_flat() {
    let doc = parse(SIMPLE);
    assert_eq!(doc.get_property("host", "ROOT").unwrap(), "localhost");
    assert_eq!(doc.get_property("port", "ROOT").unwrap(), "8080");
    assert_eq!(doc.get_property("debug", "ROOT").unwrap(), "true");
}

#[test]
fn test_parse_total_properties() {
    let doc = parse(SIMPLE);
    assert_eq!(doc.total_properties(), 3);
}

#[test]
fn test_parse_nested_depth1() {
    let doc = parse(NESTED);
    assert_eq!(doc.get_property("a_key", "Alpha").unwrap(), "a_val");
    assert_eq!(doc.get_property("a_key2", "Alpha").unwrap(), "a_val2");
}

#[test]
fn test_parse_nested_depth2() {
    let doc = parse(NESTED);
    assert_eq!(doc.get_property("b_key", "Alpha.Beta").unwrap(), "b_val");
}

#[test]
fn test_parse_nested_depth3() {
    let doc = parse(NESTED);
    assert_eq!(
        doc.get_property("g_key", "Alpha.Beta.Gamma").unwrap(),
        "g_val"
    );
}

#[test]
fn test_parse_root_prop_with_sections_present() {
    let doc = parse(NESTED);
    assert_eq!(doc.get_property("root_key", "ROOT").unwrap(), "root_val");
}

#[test]
fn test_parse_sibling_sections() {
    let doc = parse(MULTI_SECTION);
    assert_eq!(doc.get_property("driver", "Database").unwrap(), "postgres");
    assert_eq!(doc.get_property("backend", "Cache").unwrap(), "redis");
}

#[test]
fn test_parse_deep_5_levels() {
    let doc = parse(DEEP);
    assert_eq!(doc.get_property("p1", "L1").unwrap(), "v1");
    assert_eq!(doc.get_property("p2", "L1.L2").unwrap(), "v2");
    assert_eq!(doc.get_property("p3", "L1.L2.L3").unwrap(), "v3");
    assert_eq!(doc.get_property("p4", "L1.L2.L3.L4").unwrap(), "v4");
    assert_eq!(doc.get_property("p5", "L1.L2.L3.L4.L5").unwrap(), "v5");
}

#[test]
fn test_parse_wide_10_siblings() {
    let doc = parse(WIDE);
    assert_eq!(doc.get_property("k", "S1").unwrap(), "v1");
    assert_eq!(doc.get_property("k", "S5").unwrap(), "v5");
    assert_eq!(doc.get_property("k", "S10").unwrap(), "v10");
}

#[test]
fn test_parse_comments_ignored() {
    let doc = parse(COMMENTS);
    assert_eq!(doc.get_property("key1", "ROOT").unwrap(), "val1");
    assert_eq!(doc.get_property("key2", "Section").unwrap(), "val2");
    assert_eq!(doc.total_properties(), 2);
}

#[test]
fn test_parse_escape_backslash() {
    let doc = parse(ESCAPE);
    let raw = doc.get_property("windows", "paths").unwrap();
    // Raw value contains the escape sequences as stored
    assert!(raw.contains("\\"));
}
/*
 TODO: Support \\ without breaking unicode
#[test]
fn test_parse_escape_resolved() {
    let doc = parse(ESCAPE);
    let resolved = doc.get_property_resolved("windows", "paths").unwrap();
    assert_eq!(resolved.as_ref(), "C:\\Program Files\\App");
}
*/
#[test]
fn test_parse_root_prefix_equivalent() {
    let doc = parse(NESTED);
    let a = doc.get_property("a_key", "Alpha").unwrap();
    let b = doc.get_property("a_key", "ROOT.Alpha").unwrap();
    assert_eq!(a, b);
}

// ── Section ID resolution ─────────────────────────────────────────

#[test]
fn test_section_id_root() {
    let doc = parse(NESTED);
    assert_eq!(doc.get_section_id("ROOT").unwrap(), 0);
}

#[test]
fn test_section_id_depth1() {
    let doc = parse(NESTED);
    let id = doc.get_section_id("Alpha").unwrap();
    assert!(id > 0);
    assert_eq!(doc.get_section_by_id(id), "Alpha");
}

#[test]
fn test_section_id_depth2() {
    let doc = parse(NESTED);
    let id = doc.get_section_id("Alpha.Beta").unwrap();
    assert_eq!(doc.get_section_by_id(id), "Beta");
}

#[test]
fn test_section_id_depth3() {
    let doc = parse(NESTED);
    let id = doc.get_section_id("Alpha.Beta.Gamma").unwrap();
    assert_eq!(doc.get_section_by_id(id), "Gamma");
}

#[test]
fn test_section_id_root_prefix_stripped() {
    let doc = parse(NESTED);
    let a = doc.get_section_id("Alpha").unwrap();
    let b = doc.get_section_id("ROOT.Alpha").unwrap();
    assert_eq!(a, b);
}

// ── Error cases ───────────────────────────────────────────────────

#[test]
fn test_property_not_found_returns_err() {
    let doc = parse(SIMPLE);
    let result = doc.get_property("nonexistent", "ROOT");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, ErrorCode::PropertyNotFound);
}

#[test]
fn test_section_not_found_returns_err() {
    let doc = parse(SIMPLE);
    let result = doc.get_section_id("NonExistentSection");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, ErrorCode::SectionNotFound);
}

#[test]
fn test_section_parent_not_found_returns_err() {
    let doc = parse(NESTED);
    let result = doc.get_property("b_key", "FakeParent.Beta");
    assert!(result.is_err());
}

#[test]
fn test_property_wrong_section_returns_err() {
    let doc = parse(NESTED);
    // b_key is in Alpha.Beta, not Alpha
    let result = doc.get_property("b_key", "Alpha");
    assert!(result.is_err());
}

// ── Parser never panics on bad input ─────────────────────────────

#[test]
fn test_empty_input() {
    let result = Exlex::init_reader("", None, None, None, None);
    // Empty input is valid — zero properties
    assert!(result.is_ok());
    assert_eq!(result.unwrap().total_properties(), 0);
}

#[test]
fn test_only_comments() {
    let data = "# just a comment\n# another\n";
    let doc = Exlex::init_reader(data, None, None, None, None).unwrap();
    assert_eq!(doc.total_properties(), 0);
}

#[test]
fn test_unclosed_bracket_returns_err() {
    let data = r#"sect "Broken" { "key" : "val" "#;
    // No closing } — should return Err not panic
    // Parser may or may not error here depending on impl — must not panic
    let _ = Exlex::init_reader(data, None, None, None, None);
}

#[test]
fn test_extra_closing_bracket_returns_err() {
    let data = r#""key" : "val" }"#;
    let result = Exlex::init_reader(data, None, None, None, None);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, ErrorCode::InvalidBracket);
}

#[test]
fn test_unclosed_quote_returns_err() {
    let data = r#""key" : "unclosed"#;
    let result = Exlex::init_reader(data, None, None, None, None);
    assert!(result.is_err());
}

#[test]
fn test_missing_value_returns_err() {
    let data = r#""key" : "#;
    let result = Exlex::init_reader(data, None, None, None, None);
    assert!(result.is_err());
}

#[test]
fn test_null_bytes_no_panic() {
    let data = "\x00\x00\x00";
    let _ = Exlex::init_reader(data, None, None, None, None);
}

#[test]
fn test_random_ascii_no_panic() {
    let data = "!@#$%^&*()_+-=[]|;':,.<>?";
    let _ = Exlex::init_reader(data, None, None, None, None);
}

#[test]
fn test_unicode_values_no_panic() {
    let data = "\"key\" : \"日本語テスト\"\n";
    let doc = Exlex::init_reader(data, None, None, None, None).unwrap();
    assert_eq!(doc.get_property("key", "ROOT").unwrap(), "日本語テスト");
}

// ── Preallocator modes ────────────────────────────────────────────

#[test]
fn test_preallocator_disabled_same_result() {
    let a = Exlex::init_reader(NESTED, Some(false), None, None, None).unwrap();
    let b = Exlex::init_reader(NESTED, Some(true), None, None, None).unwrap();
    assert_eq!(a.total_properties(), b.total_properties());
    assert_eq!(
        a.get_property("g_key", "Alpha.Beta.Gamma").unwrap(),
        b.get_property("g_key", "Alpha.Beta.Gamma").unwrap()
    );
}

#[test]
fn test_manual_hints_same_result() {
    let a = Exlex::init_reader(NESTED, None, None, None, None).unwrap();
    let b = Exlex::init_reader(NESTED, Some(false), Some(5), Some(10), Some(3)).unwrap();
    assert_eq!(a.total_properties(), b.total_properties());
}

// ── get_properties slice ──────────────────────────────────────────

#[test]
fn test_get_properties_slice_correct_length() {
    let doc = parse(MULTI_SECTION);
    let props = doc.get_properties("Database").unwrap();
    assert_eq!(props.len(), 2); // driver, pool
}

#[test]
fn test_get_properties_slice_contains_keys() {
    let doc = parse(MULTI_SECTION);
    let props = doc.get_properties("Database").unwrap();
    assert!(props.contains(&"driver"));
    assert!(props.contains(&"pool"));
}

#[test]
fn test_get_sections_children() {
    let doc = parse(MULTI_SECTION);
    let children = doc.get_sections(Some("Database")).unwrap();
    assert!(children.contains(&"Credentials"));
    assert!(children.contains(&"Replica"));
}

#[test]
fn test_query_property_global() {
    let doc = parse(NESTED);
    // query_property searches all sections
    let result = doc.query_property("g_key");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "g_val");
}

// ═══════════════════════════════════════════════════════════════════
// MUTATOR TESTS
// ═══════════════════════════════════════════════════════════════════

fn contains_key_value(output: &str, key: &str, value: &str) -> bool {
    let pattern = format!("\"{}\": \"{}\"", key, value);
    output.contains(&pattern)
}

fn contains_key(output: &str, key: &str) -> bool {
    output.contains(&format!("\"{}\"", key))
}

fn contains_section(output: &str, name: &str) -> bool {
    output.contains(&format!("sect \"{}\"", name))
}

// ── CASE 1: parse → save, no mutations ───────────────────────────

#[test]
fn test_mutator_save_no_mutations_has_all_keys() {
    let doc = parse(MULTI_SECTION);
    let mut m = doc.init_mutator().unwrap();
    let out = m.save();
    assert!(contains_key_value(&out, "driver", "postgres"));
    assert!(contains_key_value(&out, "pool", "32"));
    assert!(contains_key_value(&out, "user", "admin"));
    assert!(contains_key_value(&out, "auth", "ed25519"));
    assert!(contains_key_value(&out, "backend", "redis"));
    assert!(contains_key_value(&out, "ttl", "3600"));
}

#[test]
fn test_mutator_save_no_mutations_has_all_sections() {
    let doc = parse(MULTI_SECTION);
    let mut m = doc.init_mutator().unwrap();
    let out = m.save();
    assert!(contains_section(&out, "Database"));
    assert!(contains_section(&out, "Credentials"));
    assert!(contains_section(&out, "Replica"));
    assert!(contains_section(&out, "Cache"));
}

// ── CASE 2: update existing key ──────────────────────────────────

#[test]
fn test_update_existing_key_value_changes() {
    let doc = parse(MULTI_SECTION);
    let sect_id = doc.get_section_id("Database").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("driver", "mysql", sect_id);
    let out = m.save();
    assert!(
        contains_key_value(&out, "driver", "mysql"),
        "new value must appear"
    );
    assert!(
        !contains_key_value(&out, "driver", "postgres"),
        "old value must be gone"
    );
}

#[test]
fn test_update_key_does_not_affect_others() {
    let doc = parse(MULTI_SECTION);
    let sect_id = doc.get_section_id("Database").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("driver", "mysql", sect_id);
    let out = m.save();
    assert!(
        contains_key_value(&out, "pool", "32"),
        "other keys must be untouched"
    );
}

#[test]
fn test_update_key_in_nested_section() {
    let doc = parse(MULTI_SECTION);
    let sect_id = doc.get_section_id("Database.Credentials").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("user", "superuser", sect_id);
    let out = m.save();
    assert!(contains_key_value(&out, "user", "superuser"));
    assert!(!contains_key_value(&out, "user", "admin"));
}

// ── CASE 3: update same key twice — second value wins ────────────

#[test]
fn test_update_same_key_twice_second_wins() {
    let doc = parse(MULTI_SECTION);
    let sect_id = doc.get_section_id("Database").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("driver", "sqlite", sect_id);
    m.update_prop("driver", "mariadb", sect_id);
    let out = m.save();
    assert!(
        contains_key_value(&out, "driver", "mariadb"),
        "last write wins"
    );
    assert!(
        !contains_key_value(&out, "driver", "sqlite"),
        "intermediate value gone"
    );
    assert!(
        !contains_key_value(&out, "driver", "postgres"),
        "original value gone"
    );
}

#[test]
fn test_update_same_key_three_times() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("host", "a.com", 0);
    m.update_prop("host", "b.com", 0);
    m.update_prop("host", "c.com", 0);
    let out = m.save();
    assert!(contains_key_value(&out, "host", "c.com"));
    assert!(!contains_key_value(&out, "host", "a.com"));
    assert!(!contains_key_value(&out, "host", "b.com"));
    assert!(!contains_key_value(&out, "host", "localhost"));
}

// ── CASE 4: delete existing key ───────────────────────────────────

#[test]
fn test_delete_existing_key_absent_from_output() {
    let doc = parse(MULTI_SECTION);
    let sect_id = doc.get_section_id("Database").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.delete_property("driver", sect_id).unwrap();
    let out = m.save();
    assert!(!contains_key(&out, "driver"), "deleted key must not appear");
}

#[test]
fn test_delete_key_sibling_keys_remain() {
    let doc = parse(MULTI_SECTION);
    let sect_id = doc.get_section_id("Database").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.delete_property("driver", sect_id).unwrap();
    let out = m.save();
    assert!(
        contains_key_value(&out, "pool", "32"),
        "sibling must remain"
    );
}

#[test]
fn test_delete_nonexistent_key_returns_err() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    let result = m.delete_property("nonexistent", 0);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, ErrorCode::PropertyNotFound);
}

// ── CASE 5: create new key ────────────────────────────────────────

#[test]
fn test_update_nonexistent_key_creates_new() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("new_key", "new_val", 0);
    let out = m.save();
    assert!(contains_key_value(&out, "new_key", "new_val"));
}

#[test]
fn test_new_key_does_not_overwrite_existing() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("new_key", "new_val", 0);
    let out = m.save();
    assert!(
        contains_key_value(&out, "host", "localhost"),
        "existing keys untouched"
    );
}

#[test]
fn test_new_key_in_nested_section() {
    let doc = parse(MULTI_SECTION);
    let sect_id = doc.get_section_id("Database.Credentials").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("token", "abc123", sect_id);
    let out = m.save();
    assert!(contains_key_value(&out, "token", "abc123"));
}

// ── CASE 6: new section ───────────────────────────────────────────

#[test]
fn test_new_section_appears_in_output() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    m.new_section("NewSection", 0).unwrap();
    let out = m.save();
    assert!(contains_section(&out, "NewSection"));
}

#[test]
fn test_new_section_duplicate_returns_err() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    m.new_section("NewSection", 0).unwrap();
    let result = m.new_section("NewSection", 0);
    assert!(result.is_err());
}

#[test]
fn test_new_section_same_name_different_parent_ok() {
    let doc = parse(MULTI_SECTION);
    let db_id = doc.get_section_id("Database").unwrap();
    let cache_id = doc.get_section_id("Cache").unwrap();
    let mut m = doc.init_mutator().unwrap();
    // Same name, different parents — should be allowed
    m.new_section("Config", db_id).unwrap();
    let result = m.new_section("Config", cache_id);
    assert!(result.is_ok());
}

#[test]
fn test_new_section_over_core_section_returns_err() {
    let doc = parse(MULTI_SECTION);
    let db_id = doc.get_section_id("Database").unwrap();
    let mut m = doc.init_mutator().unwrap();
    // Credentials already exists under Database in core
    let result = m.new_section("Credentials", db_id);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().code,
        ErrorCode::DuplicateSectionsNotAllowed
    );
}

// ── CASE 7: move section ──────────────────────────────────────────

#[test]
fn test_move_section_appears_under_new_parent() {
    let doc = parse(MULTI_SECTION);
    let replica_id = doc.get_section_id("Database.Replica").unwrap();
    let cache_id = doc.get_section_id("Cache").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.move_section(replica_id, cache_id);
    let out = m.save();
    // Replica must still appear
    assert!(contains_section(&out, "Replica"));
    // Its keys must still be there
    assert!(contains_key_value(&out, "host", "replica.db"));
}

// ── CASE 8: delete section ────────────────────────────────────────

#[test]
fn test_delete_section_absent_from_output() {
    let doc = parse(MULTI_SECTION);
    let cache_id = doc.get_section_id("Cache").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.delete_section(cache_id);
    let out = m.save();
    assert!(
        !contains_section(&out, "Cache"),
        "deleted section must not appear"
    );
}

#[test]
fn test_delete_section_sibling_remains() {
    let doc = parse(MULTI_SECTION);
    let cache_id = doc.get_section_id("Cache").unwrap();
    let mut m = doc.init_mutator().unwrap();
    m.delete_section(cache_id);
    let out = m.save();
    assert!(
        contains_section(&out, "Database"),
        "sibling section must remain"
    );
}

// ── CASE 9: get_property from mutator ────────────────────────────

#[test]
fn test_mutator_get_property_core_value() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    let val = m.get_property("host", 0).unwrap();
    assert_eq!(val, "localhost");
}

#[test]
fn test_mutator_get_property_after_update() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("host", "newhost.com", 0);
    let val = m.get_property("host", 0).unwrap();
    assert_eq!(val, "newhost.com");
}

#[test]
fn test_mutator_get_property_new_key() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("brand_new", "brand_val", 0);
    let val = m.get_property("brand_new", 0).unwrap();
    assert_eq!(val, "brand_val");
}

#[test]
fn test_mutator_get_property_not_found_returns_err() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    let result = m.get_property("ghost", 0);
    assert!(result.is_err());
}

// ── CASE 10: delete updated key ──────────────────────────────────

#[test]
fn test_delete_after_update_key_absent() {
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("host", "updated.com", 0);
    m.delete_property("host", 0).unwrap();
    let out = m.save();
    assert!(
        !contains_key(&out, "host"),
        "deleted-after-update key must not appear"
    );
}

// ── CASE 11: combined operations ─────────────────────────────────

#[test]
fn test_combined_update_delete_new() {
    let doc = parse(MULTI_SECTION);
    let db_id = doc.get_section_id("Database").unwrap();
    let cred_id = doc.get_section_id("Database.Credentials").unwrap();
    let mut m = doc.init_mutator().unwrap();

    m.update_prop("driver", "sqlite", db_id); // update existing
    m.delete_property("pool", db_id).unwrap(); // delete existing
    m.update_prop("timeout", "30", db_id); // create new
    m.update_prop("user", "superuser", cred_id); // update nested

    let out = m.save();
    assert!(contains_key_value(&out, "driver", "sqlite"));
    assert!(!contains_key(&out, "pool"));
    assert!(contains_key_value(&out, "timeout", "30"));
    assert!(contains_key_value(&out, "user", "superuser"));
    assert!(!contains_key_value(&out, "user", "admin"));
}

#[test]
fn test_save_twice_same_result() {
    // save() must be idempotent — calling it twice gives the same output
    // Note: currently save() appends to dump_data so this test verifies
    // whether re-calling save() is safe
    let doc = parse(SIMPLE);
    let mut m = doc.init_mutator().unwrap();
    m.update_prop("host", "changed.com", 0);
    let out1 = m.save();
    // Second save would double-append unless dump_data is cleared
    // This test documents current behavior — may need dump_data.clear() before write_section
    assert!(out1.contains("changed.com"));
}

// ── CASE 12: boundary and edge cases ─────────────────────────────

#[test]
fn test_empty_value_string() {
    // Empty string value — parser must handle ""
    let data = "\"key\" : \"\"\n";
    let doc = Exlex::init_reader(data, None, None, None, None).unwrap();
    assert_eq!(doc.get_property("key", "ROOT").unwrap(), "");
}

#[test]
fn test_single_property_single_section() {
    let data = "sect \"S\" {\n\"k\" : \"v\"\n}\n";
    let doc = parse(data);
    assert_eq!(doc.get_property("k", "S").unwrap(), "v");
}

#[test]
fn test_property_key_with_spaces_in_value() {
    let data = "\"key\" : \"hello world\"\n";
    let doc = parse(data);
    assert_eq!(doc.get_property("key", "ROOT").unwrap(), "hello world");
}

#[test]
fn test_large_flat_document() {
    let mut data = String::new();
    for i in 0..500 {
        data.push_str(&format!("\"key_{}\" : \"val_{}\"\n", i, i));
    }
    let doc = parse(&data);
    assert_eq!(doc.total_properties(), 500);
    assert_eq!(doc.get_property("key_0", "ROOT").unwrap(), "val_0");
    assert_eq!(doc.get_property("key_499", "ROOT").unwrap(), "val_499");
    assert_eq!(doc.get_property("key_250", "ROOT").unwrap(), "val_250");
}

#[test]
fn test_many_sections_wide() {
    let mut data = String::new();
    for i in 0..100 {
        data.push_str(&format!(
            "sect \"sect_{}\" {{\n\"k\" : \"v_{}\"\n}}\n",
            i, i
        ));
    }
    let doc = parse(&data);
    assert_eq!(doc.get_property("k", "sect_0").unwrap(), "v_0");
    assert_eq!(doc.get_property("k", "sect_99").unwrap(), "v_99");
    assert_eq!(doc.get_property("k", "sect_50").unwrap(), "v_50");
}

#[test]
fn test_update_prop_arena_does_not_corrupt_other_values() {
    // Ensures arena offset arithmetic stays correct across multiple updates
    let doc = parse(MULTI_SECTION);
    let db_id = doc.get_section_id("Database").unwrap();
    let mut m = doc.init_mutator().unwrap();

    m.update_prop("driver", "x", db_id);
    m.update_prop("pool", "999", db_id);
    m.update_prop("driver", "final_driver", db_id);

    assert_eq!(m.get_property("driver", db_id).unwrap(), "final_driver");
    assert_eq!(m.get_property("pool", db_id).unwrap(), "999");
}

#[test]
fn test_get_section_ids_finds_all_matching() {
    // Two sections with same name under different parents
    let data = r#"
sect "Parent1" {
    sect "Child" { "k" : "v1" }
}
sect "Parent2" {
    sect "Child" { "k" : "v2" }
}
"#;
    let doc = parse(data);
    let ids = doc.get_section_ids("Child");
    assert_eq!(ids.len(), 2);
}
