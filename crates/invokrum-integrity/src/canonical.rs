use invokrum_core::{OverlayPack, Profile, Sensitivity};
use serde::Serialize;

const MAX_ERROR_CHARS: usize = 256;

#[derive(Serialize)]
struct CanonicalPack {
    schema: String,
    id: String,
    classes: Vec<CanonicalClass>,
    overlays: Vec<CanonicalOverlay>,
    profiles: Vec<CanonicalProfile>,
    variables: Vec<CanonicalVariable>,
}

#[derive(Serialize)]
struct CanonicalClass {
    id: String,
    order: u32,
    minimum: u32,
    maximum: Option<u32>,
}

#[derive(Serialize)]
struct CanonicalOverlay {
    id: String,
    class: String,
    source: String,
    incompatible_with: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct CanonicalProfile {
    id: String,
    selections: Vec<CanonicalSelection>,
}

#[derive(Serialize)]
struct CanonicalSelection {
    class: String,
    overlays: Vec<String>,
}

#[derive(Serialize)]
struct CanonicalVariable {
    name: String,
    sensitivity: &'static str,
}

pub(crate) fn pack_bytes(pack: &OverlayPack) -> Result<Vec<u8>, String> {
    canonical_json(&CanonicalPack::from(pack))
}

pub(crate) fn profile_bytes(profile: &Profile) -> Result<Vec<u8>, String> {
    canonical_json(&CanonicalProfile::from(profile))
}

pub(crate) fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| {
        let message = error.to_string();
        bounded(&message)
    })
}

fn bounded(value: &str) -> String {
    let mut result: String = value.chars().take(MAX_ERROR_CHARS).collect();
    if value.chars().count() > MAX_ERROR_CHARS {
        result.push('…');
    }
    result
}

impl From<&OverlayPack> for CanonicalPack {
    fn from(pack: &OverlayPack) -> Self {
        Self {
            schema: pack.schema_family.clone(),
            id: pack.id.to_string(),
            classes: pack
                .classes()
                .iter()
                .map(|class| CanonicalClass {
                    id: class.id.to_string(),
                    order: class.order,
                    minimum: class.cardinality.minimum(),
                    maximum: class.cardinality.maximum(),
                })
                .collect(),
            overlays: pack
                .overlays()
                .iter()
                .map(|overlay| CanonicalOverlay {
                    id: overlay.id.to_string(),
                    class: overlay.class.to_string(),
                    source: overlay.source.as_str().to_owned(),
                    incompatible_with: overlay
                        .incompatible_with
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                })
                .collect(),
            profiles: pack.profiles().iter().map(CanonicalProfile::from).collect(),
            variables: pack
                .variables()
                .iter()
                .map(|variable| CanonicalVariable {
                    name: variable.name.to_string(),
                    sensitivity: match variable.sensitivity {
                        Sensitivity::Public => "public",
                        Sensitivity::Secret => "secret",
                    },
                })
                .collect(),
        }
    }
}

impl From<&Profile> for CanonicalProfile {
    fn from(profile: &Profile) -> Self {
        Self {
            id: profile.id.to_string(),
            selections: profile
                .selections
                .iter()
                .map(|(class, overlays)| CanonicalSelection {
                    class: class.to_string(),
                    overlays: overlays.iter().map(ToString::to_string).collect(),
                })
                .collect(),
        }
    }
}
