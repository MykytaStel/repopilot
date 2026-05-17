use crate::frameworks::ReactNativeArchitectureProfile;
use crate::output::report_text::tristate_label;
use std::fmt::Write;

pub(crate) fn render_react_native_section(
    output: &mut String,
    rn: &ReactNativeArchitectureProfile,
) {
    output.push_str("### React Native\n\n");

    let version = rn.react_native_version.as_deref().unwrap_or("unknown");
    writeln!(output, "- **Version:** {version}").unwrap();
    writeln!(output, "- **Project kind:** `{:?}`", rn.project_kind).unwrap();
    writeln!(
        output,
        "- **Package manager:** {}",
        rn.package_manager.as_deref().unwrap_or("unknown")
    )
    .unwrap();

    writeln!(
        output,
        "- **iOS:** {}",
        if rn.has_ios {
            "detected"
        } else {
            "not detected"
        }
    )
    .unwrap();
    writeln!(
        output,
        "- **Android:** {}",
        if rn.has_android {
            "detected"
        } else {
            "not detected"
        }
    )
    .unwrap();
    writeln!(
        output,
        "- **Android New Architecture:** {}",
        tristate_label(rn.android_new_arch_enabled)
    )
    .unwrap();
    writeln!(
        output,
        "- **iOS New Architecture:** {}",
        tristate_label(rn.ios_new_arch_enabled)
    )
    .unwrap();
    writeln!(
        output,
        "- **Expo New Architecture:** {}",
        tristate_label(rn.expo_new_arch_enabled)
    )
    .unwrap();
    writeln!(
        output,
        "- **Hermes:** {}",
        tristate_label(rn.hermes_enabled)
    )
    .unwrap();
    writeln!(
        output,
        "- **Codegen config:** {}\n",
        if rn.has_codegen_config {
            "found"
        } else {
            "missing"
        }
    )
    .unwrap();
}
