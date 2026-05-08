pub mod detector;
pub mod react_native;
pub mod types;

pub use detector::{detect_framework_projects, detect_frameworks};
pub use react_native::{
    ReactNativeArchitectureProfile, ReactNativeProjectKind, detect_react_native_architecture,
};
pub use types::{DetectedFramework, FrameworkProject};
