use serde::{Deserialize, Serialize};

/// The complete broadcast from a single repo — its public API surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Broadcast {
    pub repo: String,
    pub version: String,
    pub generated_at: String,
    pub crates: Vec<CrateSurface>,
}

/// Public surface of one crate within a repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateSurface {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<TypeDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<TraitDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<FnSig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reexports: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contracts: Vec<ContractBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    pub kind: TypeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<VariantDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generic_params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TypeKind {
    Struct,
    Enum,
    TypeAlias,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<FnSig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generic_params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnSig {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default)]
    pub is_async: bool,
}

/// Links a contract name to the items that define it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractBinding {
    pub contract: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_broadcast_yaml() {
        let broadcast = Broadcast {
            repo: "hologram".into(),
            version: "0.1.0".into(),
            generated_at: "2026-03-08T00:00:00Z".into(),
            crates: vec![CrateSurface {
                name: "hologram-core".into(),
                types: vec![TypeDef {
                    name: "Graph".into(),
                    kind: TypeKind::Struct,
                    doc: Some("The main computation graph.".into()),
                    fields: vec![FieldDef {
                        name: "nodes".into(),
                        ty: "Vec<Node>".into(),
                    }],
                    variants: vec![],
                    generic_params: vec![],
                }],
                traits: vec![],
                functions: vec![FnSig {
                    name: "compile".into(),
                    doc: None,
                    inputs: vec![("graph".into(), "&Graph".into())],
                    output: Some("Result<CompilationOutput>".into()),
                    is_async: false,
                }],
                reexports: vec![],
                contracts: vec![ContractBinding {
                    contract: "hologram-execution-plan".into(),
                    items: vec!["Graph".into(), "compile".into()],
                    doc: None,
                }],
            }],
        };

        let yaml = serde_yaml::to_string(&broadcast).unwrap();
        let parsed: Broadcast = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.repo, "hologram");
        assert_eq!(parsed.crates.len(), 1);
        assert_eq!(parsed.crates[0].types[0].name, "Graph");
        assert_eq!(
            parsed.crates[0].contracts[0].contract,
            "hologram-execution-plan"
        );
    }
}
