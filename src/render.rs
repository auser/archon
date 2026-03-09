use crate::broadcast::{CrateSurface, FnSig, TraitDef, TypeDef, TypeKind};

/// Render a crate surface as compact markdown suitable for context blocks.
pub fn render_crate_surface_compact(surface: &CrateSurface) -> String {
    let mut out = String::new();

    if !surface.types.is_empty() {
        for ty in &surface.types {
            render_type(&mut out, ty);
        }
    }

    if !surface.traits.is_empty() {
        for tr in &surface.traits {
            render_trait(&mut out, tr);
        }
    }

    if !surface.functions.is_empty() {
        for func in &surface.functions {
            render_function(&mut out, func);
        }
    }

    out
}

/// Summarize a crate surface in a single line (for dependency tables).
pub fn summarize_crate_surface(surface: &CrateSurface) -> String {
    let mut items = Vec::new();

    for ty in &surface.types {
        items.push(format!("`{}`", ty.name));
    }
    for tr in &surface.traits {
        items.push(format!("`{}`", tr.name));
    }
    for func in &surface.functions {
        items.push(format!("`{}()`", func.name));
    }

    if items.is_empty() {
        "(no public API)".to_string()
    } else if items.len() <= 5 {
        items.join(", ")
    } else {
        let first_five = &items[..5];
        format!("{}, +{} more", first_five.join(", "), items.len() - 5)
    }
}

fn render_type(out: &mut String, ty: &TypeDef) {
    let kind_label = match ty.kind {
        TypeKind::Struct => "struct",
        TypeKind::Enum => "enum",
        TypeKind::TypeAlias => "type",
    };
    let generics = if ty.generic_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", ty.generic_params.join(", "))
    };

    out.push_str(&format!("- **{}**{} ({})", ty.name, generics, kind_label));
    if let Some(doc) = &ty.doc {
        let first_line = doc.lines().next().unwrap_or("");
        out.push_str(&format!(" — {first_line}"));
    }
    out.push('\n');

    if !ty.fields.is_empty() {
        for field in &ty.fields {
            out.push_str(&format!("  - `{}: {}`\n", field.name, field.ty));
        }
    }
    if !ty.variants.is_empty() {
        for variant in &ty.variants {
            if variant.fields.is_empty() {
                out.push_str(&format!("  - `{}`\n", variant.name));
            } else {
                let fields: Vec<String> = variant
                    .fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name, f.ty))
                    .collect();
                out.push_str(&format!("  - `{}({})`\n", variant.name, fields.join(", ")));
            }
        }
    }
}

fn render_trait(out: &mut String, tr: &TraitDef) {
    let generics = if tr.generic_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", tr.generic_params.join(", "))
    };
    out.push_str(&format!("- **{}**{}", tr.name, generics));
    if let Some(doc) = &tr.doc {
        let first_line = doc.lines().next().unwrap_or("");
        out.push_str(&format!(" — {first_line}"));
    }
    out.push('\n');

    for method in &tr.methods {
        let async_prefix = if method.is_async { "async " } else { "" };
        let params: Vec<String> = method
            .inputs
            .iter()
            .map(|(name, ty)| format!("{name}: {ty}"))
            .collect();
        let ret = method
            .output
            .as_deref()
            .map(|r| format!(" -> {r}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "  - `{async_prefix}fn {}({}){ret}`\n",
            method.name,
            params.join(", ")
        ));
    }
}

fn render_function(out: &mut String, func: &FnSig) {
    let async_prefix = if func.is_async { "async " } else { "" };
    let params: Vec<String> = func
        .inputs
        .iter()
        .map(|(name, ty)| format!("{name}: {ty}"))
        .collect();
    let ret = func
        .output
        .as_deref()
        .map(|r| format!(" -> {r}"))
        .unwrap_or_default();

    out.push_str(&format!(
        "- `{async_prefix}fn {}({}){ret}`",
        func.name,
        params.join(", ")
    ));
    if let Some(doc) = &func.doc {
        let first_line = doc.lines().next().unwrap_or("");
        out.push_str(&format!(" — {first_line}"));
    }
    out.push('\n');
}
