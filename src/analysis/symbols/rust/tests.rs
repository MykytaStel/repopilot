use super::extract;
use crate::analysis::parse::{ParseLanguage, parse};

#[test]
fn grouped_rust_imports_retain_exported_and_aliased_names() {
    let source = concat!(
        "mod api;\n",
        "pub struct Model { pub id: u32 }\n",
        "use crate::api::{run, Model as Shape};\n",
        "fn call() { api::run(); let model = api::Model { id: 1 }; }\n",
    );
    let tree = parse(source, ParseLanguage::Rust).expect("valid Rust");
    let facts = extract(source, &tree).expect("bounded Rust facts");

    assert!(facts.imports.iter().any(|import| {
        import.imported_name == "run"
            && import.local_name == "run"
            && import.module_specifier == "crate::api"
    }));
    assert!(facts.imports.iter().any(|import| {
        import.imported_name == "Model"
            && import.local_name == "Shape"
            && import.module_specifier == "crate::api"
    }));
    assert!(facts.rust_contracts.as_ref().is_some_and(|contracts| {
        contracts
            .calls
            .iter()
            .any(|call| call.imported_name == "run")
            && contracts
                .public_struct_fields
                .iter()
                .any(|field| field.struct_name == "Model" && field.field_name == "id")
            && contracts.struct_field_uses.iter().any(|field| {
                field.struct_name == "Model"
                    && field.field_name == "id"
                    && field.module_specifier == "mod::api"
            })
    }));
}

#[test]
fn public_enum_facts_capture_variants_and_local_qualified_uses() {
    let source = concat!(
        "mod api;\n",
        "pub enum State { Ready, Waiting(u8), Failed { code: i32 } }\n",
        "fn run(value: api::State) {\n",
        "    let _ = api::State::Ready;\n",
        "    let _ = api::State::Waiting(1);\n",
        "    match value { api::State::Failed { code } => {}, _ => {} }\n",
        "}\n",
    );
    let tree = parse(source, ParseLanguage::Rust).expect("valid Rust");
    let facts = extract(source, &tree).expect("bounded Rust facts");
    let contracts = facts.rust_contracts.expect("Rust contract facts");

    assert_eq!(contracts.public_enums, ["State"]);
    assert_eq!(
        contracts
            .public_enum_variants
            .iter()
            .map(|variant| variant.variant_name.as_str())
            .collect::<Vec<_>>(),
        ["Failed", "Ready", "Waiting"]
    );
    assert!(
        contracts
            .public_enum_variants
            .iter()
            .all(|variant| variant.line_start == 2 && variant.line_end == 2)
    );
    assert_eq!(
        contracts
            .enum_variant_uses
            .iter()
            .map(|usage| usage.variant_name.as_str())
            .collect::<Vec<_>>(),
        ["Failed", "Ready", "Waiting"]
    );
}

#[test]
fn nested_rust_module_paths_capture_enum_uses_and_imports() {
    let source = concat!(
        "mod api;\n",
        "use crate::api::types::State;\n",
        "fn run() { let _ = crate::api::types::State::Old; }\n",
    );
    let tree = parse(source, ParseLanguage::Rust).expect("valid Rust");
    let facts = extract(source, &tree).expect("bounded Rust facts");
    let contracts = facts.rust_contracts.expect("Rust contract facts");

    assert!(facts.imports.iter().any(|import| {
        import.imported_name == "State" && import.module_specifier == "crate::api::types"
    }));
    assert_eq!(
        contracts.enum_variant_uses.len(),
        1,
        "{:?}",
        contracts.enum_variant_uses
    );
    assert_eq!(contracts.enum_variant_uses[0].enum_name, "State");
    assert_eq!(contracts.enum_variant_uses[0].variant_name, "Old");
    assert_eq!(
        contracts.enum_variant_uses[0].module_specifier,
        "crate::api::types"
    );
}

#[test]
fn imported_enum_aliases_resolve_variant_uses_to_the_imported_type() {
    let source = concat!(
        "mod api;\n",
        "use crate::api::State as Status;\n",
        "fn run() { let _ = Status::Old; }\n",
    );
    let tree = parse(source, ParseLanguage::Rust).expect("valid Rust");
    let facts = extract(source, &tree).expect("bounded Rust facts");
    let contracts = facts.rust_contracts.expect("Rust contract facts");

    assert_eq!(contracts.enum_variant_uses.len(), 1);
    assert_eq!(contracts.enum_variant_uses[0].enum_name, "State");
    assert_eq!(contracts.enum_variant_uses[0].variant_name, "Old");
    assert_eq!(
        contracts.enum_variant_uses[0].module_specifier,
        "crate::api"
    );
}

#[test]
fn cached_enum_variant_facts_without_new_spans_remain_readable() {
    let old = serde_json::json!({
        "enum_name": "State",
        "variant_name": "Ready"
    });
    let fact: super::super::RustEnumVariantFact =
        serde_json::from_value(old).expect("pre-span cached facts deserialize");

    assert_eq!(fact.enum_name, "State");
    assert_eq!(fact.variant_name, "Ready");
    assert_eq!((fact.line_start, fact.line_end), (0, 0));
}
