use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use syn::{
    Expr, Fields, GenericParam, Item, ItemEnum, ItemFn, ItemStruct, ItemTrait, ItemType,
    ReturnType, TraitItem, Type, Visibility,
};

use crate::broadcast::{
    ContractBinding, CrateSurface, FieldDef, FnSig, TraitDef, TypeDef, TypeKind, VariantDef,
};

/// Extract the public API surface of a crate from its source directory.
#[allow(dead_code)]
pub fn extract_crate_surface(crate_name: &str, src_dir: &Path) -> Result<CrateSurface> {
    extract_crate_surface_with_contracts(crate_name, src_dir)
}

/// Extract a crate surface and also populate contract bindings
/// from `@contract` annotations found in source.
pub fn extract_crate_surface_with_contracts(
    crate_name: &str,
    src_dir: &Path,
) -> Result<CrateSurface> {
    let mut surface = CrateSurface {
        name: crate_name.to_string(),
        types: vec![],
        traits: vec![],
        functions: vec![],
        reexports: vec![],
        contracts: vec![],
    };

    let entry_point = find_entry_point(src_dir)?;
    let mut visited = Vec::new();
    extract_from_file_with_contracts(&entry_point, src_dir, &mut surface, &mut visited)?;

    surface.types.sort_by(|a, b| a.name.cmp(&b.name));
    surface.traits.sort_by(|a, b| a.name.cmp(&b.name));
    surface.functions.sort_by(|a, b| a.name.cmp(&b.name));
    surface.reexports.sort();
    deduplicate_contracts(&mut surface);

    Ok(surface)
}

fn find_entry_point(src_dir: &Path) -> Result<PathBuf> {
    let lib_rs = src_dir.join("lib.rs");
    if lib_rs.exists() {
        return Ok(lib_rs);
    }
    let main_rs = src_dir.join("main.rs");
    if main_rs.exists() {
        return Ok(main_rs);
    }
    anyhow::bail!("no lib.rs or main.rs found in {}", src_dir.display());
}

fn is_pub(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn extract_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let syn::Lit::Str(s) = &expr_lit.lit {
                        lines.push(s.value().trim().to_string());
                    }
                }
            }
        }
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn extract_contract_annotations(attrs: &[syn::Attribute]) -> Vec<String> {
    let doc = match extract_doc_comment(attrs) {
        Some(d) => d,
        None => return vec![],
    };
    let mut contracts = Vec::new();
    for line in doc.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@contract(") {
            if let Some(name) = rest.strip_suffix(')') {
                contracts.push(name.trim().to_string());
            }
        }
    }
    contracts
}

fn extract_struct(s: &ItemStruct) -> TypeDef {
    let doc = extract_doc_comment(&s.attrs);
    let fields = match &s.fields {
        Fields::Named(named) => named
            .named
            .iter()
            .filter(|f| is_pub(&f.vis))
            .map(|f| FieldDef {
                name: f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default(),
                ty: type_to_string(&f.ty),
            })
            .collect(),
        _ => vec![],
    };
    let generic_params = extract_generics(&s.generics);

    TypeDef {
        name: s.ident.to_string(),
        kind: TypeKind::Struct,
        doc: strip_contract_annotations(doc),
        fields,
        variants: vec![],
        generic_params,
    }
}

fn extract_enum(e: &ItemEnum) -> TypeDef {
    let doc = extract_doc_comment(&e.attrs);
    let variants = e
        .variants
        .iter()
        .map(|v| {
            let fields = match &v.fields {
                Fields::Named(named) => named
                    .named
                    .iter()
                    .map(|f| FieldDef {
                        name: f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default(),
                        ty: type_to_string(&f.ty),
                    })
                    .collect(),
                Fields::Unnamed(unnamed) => unnamed
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, f)| FieldDef {
                        name: format!("{i}"),
                        ty: type_to_string(&f.ty),
                    })
                    .collect(),
                Fields::Unit => vec![],
            };
            VariantDef {
                name: v.ident.to_string(),
                fields,
            }
        })
        .collect();
    let generic_params = extract_generics(&e.generics);

    TypeDef {
        name: e.ident.to_string(),
        kind: TypeKind::Enum,
        doc: strip_contract_annotations(doc),
        fields: vec![],
        variants,
        generic_params,
    }
}

fn extract_trait(t: &ItemTrait) -> TraitDef {
    let doc = extract_doc_comment(&t.attrs);
    let methods = t
        .items
        .iter()
        .filter_map(|item| {
            if let TraitItem::Fn(method) = item {
                Some(extract_trait_method(method))
            } else {
                None
            }
        })
        .collect();
    let generic_params = extract_generics(&t.generics);

    TraitDef {
        name: t.ident.to_string(),
        doc: strip_contract_annotations(doc),
        methods,
        generic_params,
    }
}

fn extract_trait_method(method: &syn::TraitItemFn) -> FnSig {
    let doc = extract_doc_comment(&method.attrs);
    let sig = &method.sig;
    let inputs = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pat_type) => {
                let name = pat_to_string(&pat_type.pat);
                let ty = type_to_string(&pat_type.ty);
                Some((name, ty))
            }
            syn::FnArg::Receiver(_) => None,
        })
        .collect();
    let output = match &sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some(type_to_string(ty)),
    };

    FnSig {
        name: sig.ident.to_string(),
        doc,
        inputs,
        output,
        is_async: sig.asyncness.is_some(),
    }
}

fn extract_fn(f: &ItemFn) -> FnSig {
    let doc = extract_doc_comment(&f.attrs);
    let sig = &f.sig;
    let inputs = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pat_type) => {
                let name = pat_to_string(&pat_type.pat);
                let ty = type_to_string(&pat_type.ty);
                Some((name, ty))
            }
            syn::FnArg::Receiver(_) => None,
        })
        .collect();
    let output = match &sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some(type_to_string(ty)),
    };

    FnSig {
        name: sig.ident.to_string(),
        doc: strip_contract_annotations(doc),
        inputs,
        output,
        is_async: sig.asyncness.is_some(),
    }
}

fn extract_type_alias(t: &ItemType) -> TypeDef {
    let doc = extract_doc_comment(&t.attrs);
    let generic_params = extract_generics(&t.generics);

    TypeDef {
        name: t.ident.to_string(),
        kind: TypeKind::TypeAlias,
        doc: strip_contract_annotations(doc),
        fields: vec![],
        variants: vec![],
        generic_params,
    }
}

fn extract_generics(generics: &syn::Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .map(|p| match p {
            GenericParam::Type(tp) => tp.ident.to_string(),
            GenericParam::Lifetime(lt) => format!("'{}", lt.lifetime.ident),
            GenericParam::Const(c) => format!("const {}", c.ident),
        })
        .collect()
}

fn type_to_string(ty: &Type) -> String {
    quote::quote!(#ty)
        .to_string()
        .replace(' ', "")
        .replace(',', ", ")
        .replace("->", " -> ")
}

fn pat_to_string(pat: &syn::Pat) -> String {
    quote::quote!(#pat).to_string()
}

fn format_use_tree(tree: &syn::UseTree) -> String {
    quote::quote!(#tree).to_string()
}

fn resolve_mod_path(current_file: &Path, src_dir: &Path, mod_name: &str) -> Option<PathBuf> {
    let parent = current_file.parent()?;

    let file_stem = current_file.file_stem()?.to_str()?;
    let search_dir = if file_stem == "lib" || file_stem == "main" || file_stem == "mod" {
        parent.to_path_buf()
    } else {
        parent.join(file_stem)
    };

    let direct = search_dir.join(format!("{mod_name}.rs"));
    if direct.exists() {
        return Some(direct);
    }
    let nested = search_dir.join(mod_name).join("mod.rs");
    if nested.exists() {
        return Some(nested);
    }

    let from_root = src_dir.join(format!("{mod_name}.rs"));
    if from_root.exists() {
        return Some(from_root);
    }
    let from_root_nested = src_dir.join(mod_name).join("mod.rs");
    if from_root_nested.exists() {
        return Some(from_root_nested);
    }

    None
}

fn strip_contract_annotations(doc: Option<String>) -> Option<String> {
    let doc = doc?;
    let cleaned: Vec<&str> = doc
        .lines()
        .filter(|line| !line.trim().starts_with("@contract("))
        .collect();
    let result = cleaned.join("\n").trim().to_string();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn deduplicate_contracts(surface: &mut CrateSurface) {
    use std::collections::HashMap;

    let existing = std::mem::take(&mut surface.contracts);
    let mut contract_map: HashMap<String, Vec<String>> = HashMap::new();
    for binding in existing {
        contract_map
            .entry(binding.contract.clone())
            .or_default()
            .extend(binding.items);
    }

    surface.contracts = contract_map
        .into_iter()
        .map(|(contract, mut items)| {
            items.sort();
            items.dedup();
            ContractBinding {
                contract,
                items,
                doc: None,
            }
        })
        .collect();
    surface
        .contracts
        .sort_by(|a, b| a.contract.cmp(&b.contract));
}

fn extract_from_file_with_contracts(
    file_path: &Path,
    src_dir: &Path,
    surface: &mut CrateSurface,
    visited: &mut Vec<PathBuf>,
) -> Result<()> {
    let canonical = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf());
    if visited.contains(&canonical) {
        return Ok(());
    }
    visited.push(canonical);

    let source = std::fs::read_to_string(file_path)
        .with_context(|| format!("reading {}", file_path.display()))?;
    let syntax =
        syn::parse_file(&source).with_context(|| format!("parsing {}", file_path.display()))?;

    for item in &syntax.items {
        extract_item_with_contracts(item, file_path, src_dir, surface, visited)?;
    }

    Ok(())
}

fn extract_item_with_contracts(
    item: &Item,
    file_path: &Path,
    src_dir: &Path,
    surface: &mut CrateSurface,
    visited: &mut Vec<PathBuf>,
) -> Result<()> {
    match item {
        Item::Struct(s) if is_pub(&s.vis) => {
            let contracts = extract_contract_annotations(&s.attrs);
            let name = s.ident.to_string();
            surface.types.push(extract_struct(s));
            for c in contracts {
                surface.contracts.push(ContractBinding {
                    contract: c,
                    items: vec![name.clone()],
                    doc: None,
                });
            }
        }
        Item::Enum(e) if is_pub(&e.vis) => {
            let contracts = extract_contract_annotations(&e.attrs);
            let name = e.ident.to_string();
            surface.types.push(extract_enum(e));
            for c in contracts {
                surface.contracts.push(ContractBinding {
                    contract: c,
                    items: vec![name.clone()],
                    doc: None,
                });
            }
        }
        Item::Trait(t) if is_pub(&t.vis) => {
            let contracts = extract_contract_annotations(&t.attrs);
            let name = t.ident.to_string();
            surface.traits.push(extract_trait(t));
            for c in contracts {
                surface.contracts.push(ContractBinding {
                    contract: c,
                    items: vec![name.clone()],
                    doc: None,
                });
            }
        }
        Item::Fn(f) if is_pub(&f.vis) => {
            let contracts = extract_contract_annotations(&f.attrs);
            let name = f.sig.ident.to_string();
            surface.functions.push(extract_fn(f));
            for c in contracts {
                surface.contracts.push(ContractBinding {
                    contract: c,
                    items: vec![name.clone()],
                    doc: None,
                });
            }
        }
        Item::Type(t) if is_pub(&t.vis) => {
            let contracts = extract_contract_annotations(&t.attrs);
            let name = t.ident.to_string();
            surface.types.push(extract_type_alias(t));
            for c in contracts {
                surface.contracts.push(ContractBinding {
                    contract: c,
                    items: vec![name.clone()],
                    doc: None,
                });
            }
        }
        Item::Use(u) if is_pub(&u.vis) => {
            let reexport = format_use_tree(&u.tree);
            surface.reexports.push(reexport);
        }
        Item::Mod(m) if is_pub(&m.vis) => {
            if let Some((_, items)) = &m.content {
                for sub_item in items {
                    extract_item_with_contracts(sub_item, file_path, src_dir, surface, visited)?;
                }
            } else {
                let mod_name = m.ident.to_string();
                if let Some(mod_path) = resolve_mod_path(file_path, src_dir, &mod_name) {
                    extract_from_file_with_contracts(&mod_path, src_dir, surface, visited)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn extract_simple_struct() {
        let dir = tempfile::tempdir().unwrap();
        let lib_rs = dir.path().join("lib.rs");
        fs::write(
            &lib_rs,
            r#"
/// A graph node.
///
/// @contract(my-contract)
pub struct Node {
    pub id: u64,
    pub label: String,
    internal: Vec<u8>,
}

pub enum Color {
    Red,
    Green,
    Blue,
}

pub fn process(node: &Node) -> bool {
    true
}

fn private_fn() {}
struct PrivateStruct;
"#,
        )
        .unwrap();

        let surface = extract_crate_surface_with_contracts("test-crate", dir.path()).unwrap();
        assert_eq!(surface.types.len(), 2);
        assert_eq!(surface.functions.len(), 1);

        let node = surface.types.iter().find(|t| t.name == "Node").unwrap();
        assert_eq!(node.kind, TypeKind::Struct);
        assert_eq!(node.fields.len(), 2);
        assert!(node.doc.as_ref().unwrap().contains("A graph node"));
        assert!(!node.doc.as_ref().unwrap().contains("@contract"));

        let color = surface.types.iter().find(|t| t.name == "Color").unwrap();
        assert_eq!(color.kind, TypeKind::Enum);
        assert_eq!(color.variants.len(), 3);

        assert_eq!(surface.contracts.len(), 1);
        assert_eq!(surface.contracts[0].contract, "my-contract");
        assert_eq!(surface.contracts[0].items, vec!["Node"]);
    }

    #[test]
    fn extract_trait_and_type_alias() {
        let dir = tempfile::tempdir().unwrap();
        let lib_rs = dir.path().join("lib.rs");
        fs::write(
            &lib_rs,
            r#"
pub trait Executor {
    fn execute(&self, plan: &Plan) -> Result<Output>;
    async fn execute_async(&self, plan: &Plan) -> Result<Output>;
}

pub type PlanResult = Result<Output, Error>;
"#,
        )
        .unwrap();

        let surface = extract_crate_surface("test-crate", dir.path()).unwrap();
        assert_eq!(surface.traits.len(), 1);
        assert_eq!(surface.traits[0].name, "Executor");
        assert_eq!(surface.traits[0].methods.len(), 2);
        assert!(!surface.traits[0].methods[0].is_async);
        assert!(surface.traits[0].methods[1].is_async);

        assert_eq!(surface.types.len(), 1);
        assert_eq!(surface.types[0].kind, TypeKind::TypeAlias);
    }

    #[test]
    fn extract_follows_pub_modules() {
        let dir = tempfile::tempdir().unwrap();
        let lib_rs = dir.path().join("lib.rs");
        fs::write(&lib_rs, "pub mod sub;\n").unwrap();

        let sub_rs = dir.path().join("sub.rs");
        fs::write(
            &sub_rs,
            r#"
pub struct SubItem {
    pub value: i32,
}
"#,
        )
        .unwrap();

        let surface = extract_crate_surface("test-crate", dir.path()).unwrap();
        assert_eq!(surface.types.len(), 1);
        assert_eq!(surface.types[0].name, "SubItem");
    }

    #[test]
    fn extract_skips_private_modules() {
        let dir = tempfile::tempdir().unwrap();
        let lib_rs = dir.path().join("lib.rs");
        fs::write(
            &lib_rs,
            r#"
mod private_mod;
pub struct PublicThing;
"#,
        )
        .unwrap();

        let private_rs = dir.path().join("private_mod.rs");
        fs::write(&private_rs, "pub struct ShouldNotAppear;\n").unwrap();

        let surface = extract_crate_surface("test-crate", dir.path()).unwrap();
        assert_eq!(surface.types.len(), 1);
        assert_eq!(surface.types[0].name, "PublicThing");
    }
}
