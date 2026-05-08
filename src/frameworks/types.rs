use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "name", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DetectedFramework {
    ReactNative { version: Option<String> },
    Expo { version: Option<String> },
    NextJs { version: Option<String> },
    React { version: Option<String> },
    Vue { version: Option<String> },
    Angular { version: Option<String> },
    Svelte { version: Option<String> },
    NestJs { version: Option<String> },
    Express { version: Option<String> },
}

impl DetectedFramework {
    pub fn label(&self) -> String {
        let (name, version) = match self {
            DetectedFramework::ReactNative { version } => ("React Native", version.as_deref()),
            DetectedFramework::Expo { version } => ("Expo", version.as_deref()),
            DetectedFramework::NextJs { version } => ("Next.js", version.as_deref()),
            DetectedFramework::React { version } => ("React", version.as_deref()),
            DetectedFramework::Vue { version } => ("Vue", version.as_deref()),
            DetectedFramework::Angular { version } => ("Angular", version.as_deref()),
            DetectedFramework::Svelte { version } => ("Svelte", version.as_deref()),
            DetectedFramework::NestJs { version } => ("NestJS", version.as_deref()),
            DetectedFramework::Express { version } => ("Express", version.as_deref()),
        };
        match version {
            Some(v) => format!("{name} {v}"),
            None => name.to_string(),
        }
    }
}
