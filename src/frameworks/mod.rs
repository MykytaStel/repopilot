pub mod detector;
pub mod react_native;
pub mod types;

pub use detector::detect_frameworks;
pub use react_native::{ReactNativeArchitectureProfile, detect_react_native_architecture};
pub use types::DetectedFramework;
