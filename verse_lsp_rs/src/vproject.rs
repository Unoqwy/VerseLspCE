use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VProjectFile {
    pub packages: Vec<VProjectPackage>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VProjectPackage {
    pub desc: PackageDesc,
    pub read_only: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageDesc {
    pub name: String,
    pub dir_path: String,
    pub settings: PackageSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageSettings {
    pub verse_path: String,
    pub verse_scope: VerseScope,
    #[serde(default = "PackageRole::source")]
    pub role: PackageRole,
    pub verse_version: Option<u32>,
    #[serde(default)]
    pub treat_modules_as_implicit: bool,
    pub dependency_packages: Vec<String>,
    pub vni_dest_dir: Option<String>,
    #[serde(default)]
    pub allow_experimental: bool,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum VerseScope {
    PublicAPI,
    InternalAPI,
    PublicUser,
    InternalUser,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum PackageRole {
    Source,
    External,
    GenerateCompatConstraint,
    PersistenceCompatConstraint,
    PersistenceSoftCompatConstraint,
}

impl PackageRole {
    fn source() -> Self {
        Self::Source
    }
}
