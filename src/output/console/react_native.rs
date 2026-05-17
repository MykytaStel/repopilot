use crate::frameworks::ReactNativeArchitectureProfile;
use crate::output::report_text::tristate_label;
use std::fmt::Write;

pub(crate) fn render_react_native_section(
    output: &mut String,
    rn: &ReactNativeArchitectureProfile,
) {
    let version = rn.react_native_version.as_deref().unwrap_or("unknown");
    let ios = if rn.has_ios { "yes" } else { "no" };
    let android = if rn.has_android { "yes" } else { "no" };
    let new_arch_android = tristate_label(rn.android_new_arch_enabled);
    let new_arch_ios = tristate_label(rn.ios_new_arch_enabled);
    let new_arch_expo = tristate_label(rn.expo_new_arch_enabled);
    let hermes = tristate_label(rn.hermes_enabled);
    let codegen = if rn.has_codegen_config { "yes" } else { "no" };
    let package_manager = rn.package_manager.as_deref().unwrap_or("unknown");

    output.push_str("React Native:\n");
    writeln!(
        output,
        "  Version: {version}  Kind: {:?}  Package manager: {package_manager}",
        rn.project_kind
    )
    .unwrap();
    writeln!(
        output,
        "  iOS: {ios}  Android: {android}  Expo config: {}",
        if rn.has_expo_config { "yes" } else { "no" }
    )
    .unwrap();
    writeln!(
        output,
        "  New Arch (Android): {new_arch_android}  New Arch (iOS): {new_arch_ios}  New Arch (Expo): {new_arch_expo}"
    )
    .unwrap();
    writeln!(output, "  Hermes: {hermes}  Codegen: {codegen}\n").unwrap();
}
