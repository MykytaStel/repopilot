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
