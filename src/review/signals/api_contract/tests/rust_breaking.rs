use super::rust::rust_case;
use super::*;

#[test]
fn rust_removed_public_function_supports_grouped_imports() {
    let signals = rust_case(
        "pub fn load() {}",
        "pub fn save() {}",
        "mod api; use api::{load as fetch}; fn run() { fetch(); }",
        false,
    );
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].exported_name, "load");
    assert_eq!(signals[0].local_name, "fetch");
}

#[test]
fn rust_arity_controls_require_a_previously_matching_direct_call() {
    for (before, after, caller) in [
        (
            "pub fn load(user: usize) {}",
            "pub fn load(user: usize, mode: bool) {}",
            "mod api; fn run() { api::load(1, true); }",
        ),
        (
            "pub fn load(user: usize) {}",
            "pub fn load(user: String) {}",
            "mod api; fn run() { api::load(1); }",
        ),
        (
            "pub fn load<T>(user: T) {}",
            "pub fn load<T>(user: T, mode: bool) {}",
            "mod api; fn run() { api::load::<usize>(1); }",
        ),
        (
            "pub fn load(user: usize) {}",
            "pub fn load(user: usize, mode: bool) {}",
            "mod api; use api::load as fetch; fn run() { fetch(1); }",
        ),
        (
            "pub fn load(user: usize) {}",
            "pub fn save(user: usize, mode: bool) {}",
            "mod api; fn run() { api::load(1); }",
        ),
    ] {
        let signals = rust_case(before, after, caller, false);
        assert!(
            signals
                .iter()
                .all(|signal| !matches!(signal.change, ContractChange::RustFunctionArity { .. })),
            "{before} -> {after}: {caller}"
        );
    }
}

#[test]
fn rust_removed_public_struct_field_flags_a_surviving_literal_field() {
    let signals = rust_case(
        "pub struct Config { pub old: usize, pub kept: bool }",
        "pub struct Config { pub kept: bool }",
        "mod api; fn run() { let config = api::Config { old: 1, kept: true }; }",
        false,
    );

    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].exported_name, "old");
    assert_eq!(signals[0].module_specifier, "mod::api");
    assert_eq!(signals[0].change, ContractChange::RustStructFieldRemoved);
}

#[test]
fn rust_removed_public_enum_variant_flags_surviving_constructors_and_matches() {
    let signals = rust_case(
        "pub enum State { Ready, Waiting(u8), Failed { code: i32 } }",
        "pub enum State { Ready, Failed { code: i32 } }",
        "mod api; fn run(state: api::State) { let _ = api::State::Waiting(1); match state { api::State::Waiting(value) => {}, _ => {} } }",
        false,
    );

    assert_eq!(signals.len(), 2);
    assert!(signals.iter().all(|signal| {
        signal.exported_name == "Waiting"
            && signal.module_specifier == "mod::api"
            && signal.change == ContractChange::RustEnumVariantRemoved
            && signal.declaration_lines == Some((1, 1))
    }));
}

#[test]
fn rust_removed_enum_variant_resolves_an_imported_type_alias() {
    let signals = rust_case(
        "pub enum State { Old, Ready }",
        "pub enum State { Ready }",
        "mod api; use crate::api::State as Status; fn run() { let _ = Status::Old; }",
        false,
    );

    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].exported_name, "Old");
    assert_eq!(signals[0].local_name, "State");
    assert_eq!(signals[0].module_specifier, "crate::api");
}

#[test]
fn rust_removed_unit_and_struct_enum_variants_flag_surviving_constructors() {
    let signals = rust_case(
        "pub enum State { Old, Failed { code: i32 }, Ready }",
        "pub enum State { Ready }",
        "mod api; fn run() { let _ = api::State::Old; let _ = api::State::Failed { code: 1 }; }",
        false,
    );

    assert_eq!(signals.len(), 2);
    assert!(signals.iter().all(|signal| {
        signal.module_specifier == "mod::api"
            && signal.change == ContractChange::RustEnumVariantRemoved
    }));
    assert_eq!(
        signals
            .iter()
            .map(|signal| signal.exported_name.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["Failed", "Old"])
    );
}

#[test]
fn rust_enum_variant_contract_ignores_other_enums_and_coordinated_changes() {
    for (before, after, caller) in [
        (
            "pub enum State { Old, Other }",
            "pub enum State { Other }",
            "mod api; fn run() { let _ = api::Other::Old; }",
        ),
        (
            "pub enum State { Old, Other }",
            "pub enum State { Old, Other }",
            "mod api; fn run() { let _ = api::State::Other; }",
        ),
        (
            "enum State { Old, Other }",
            "enum State { Other }",
            "mod api; fn run() { let _ = api::State::Old; }",
        ),
        (
            "#[cfg(feature = \"old\")] pub enum State { Old, Other }",
            "#[cfg(feature = \"new\")] pub enum State { Other }",
            "mod api; fn run() { let _ = api::State::Old; }",
        ),
    ] {
        assert!(
            rust_case(before, after, caller, false).is_empty(),
            "{before} -> {after}: {caller}"
        );
    }
}

#[test]
fn rust_struct_field_contract_preserves_private_and_coordinated_cases() {
    for (before, after, caller) in [
        (
            "pub struct Config { old: usize }",
            "pub struct Config { new: usize }",
            "mod api; fn run() { let config = api::Config { old: 1 }; }",
        ),
        (
            "pub struct Config { pub old: usize }",
            "pub struct Config { pub new: usize }",
            "mod api; fn run() { let config = api::Config { new: 1 }; }",
        ),
    ] {
        assert!(
            rust_case(before, after, caller, false).is_empty(),
            "{before} -> {after}: {caller}"
        );
    }
}

#[test]
fn v028_rust_variant_ignores_shadowed_type_and_compatible_replacement() {
    for (after, caller) in [
        (
            "pub enum State { Ready }",
            "mod api; use api::State as Status; fn run() { enum Status { Old } let _ = Status::Old; }",
        ),
        (
            "pub enum State { Ready } impl State { pub const Old: Self = Self::Ready; }",
            "mod api; fn run() { let _ = api::State::Old; }",
        ),
    ] {
        assert!(rust_case("pub enum State { Old, Ready }", after, caller, false).is_empty());
    }
}
