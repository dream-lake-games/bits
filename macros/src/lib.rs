extern crate proc_macro;

mod aseprite;

use anyhow::Context;
use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::{Data, DeriveInput, Fields, Lit, Meta, Token, Variant, parse_macro_input};

#[proc_macro_derive(Assemblable, attributes(file, tag, exclude_prefix))]
pub fn derive_assemblable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    if !is_unit_struct(&input) {
        return syn::Error::new_spanned(
            &input.ident,
            "Assemblable can only be derived for unit structs",
        )
        .to_compile_error()
        .into();
    }

    let (file, tag, exclude_prefix) = match parse_attributes(&input) {
        Ok(attrs) => attrs,
        Err(e) => return e.to_compile_error().into(),
    };

    let absolute_file = match get_absolute_path(&file) {
        Ok(p) => p,
        Err(e) => {
            return syn::Error::new_spanned(&input.ident, format!("{}", e))
                .to_compile_error()
                .into();
        }
    };

    let pixel_locations =
        match extract_pixel_locations(&absolute_file, &tag, exclude_prefix.as_deref()) {
            Ok(locs) => locs,
            Err(e) => {
                return syn::Error::new_spanned(
                    &input.ident,
                    format!("Failed to extract pixel locations: {}", e),
                )
                .to_compile_error()
                .into();
            }
        };

    let struct_name = &input.ident;
    let pixel_exprs: Vec<_> = pixel_locations
        .iter()
        .map(|(x, y)| quote! { bevy::prelude::IVec2::new(#x, #y) })
        .collect();

    let expanded = quote! {
        impl Assemblable for #struct_name {
            fn get_pixel_locations() -> Vec<bevy::prelude::IVec2> {
                const _: &[u8] = include_bytes!(#absolute_file);
                vec![#(#pixel_exprs),*]
            }
        }
    };

    TokenStream::from(expanded)
}

fn is_unit_struct(input: &DeriveInput) -> bool {
    matches!(
        &input.data,
        Data::Struct(data) if matches!(&data.fields, Fields::Unit)
    )
}

fn parse_attributes(input: &DeriveInput) -> syn::Result<(String, String, Option<String>)> {
    let mut file: Option<String> = None;
    let mut tag: Option<String> = None;
    let mut exclude_prefix: Option<String> = None;

    for attr in &input.attrs {
        let value = extract_string_attr(attr)?;

        if attr.path().is_ident("file") {
            file = value;
        } else if attr.path().is_ident("tag") {
            tag = value;
        } else if attr.path().is_ident("exclude_prefix") {
            exclude_prefix = value;
        }
    }

    let file = file.ok_or_else(|| {
        syn::Error::new_spanned(&input.ident, "Missing required attribute: #[file(\"...\")]")
    })?;

    let tag = tag.ok_or_else(|| {
        syn::Error::new_spanned(&input.ident, "Missing required attribute: #[tag(\"...\")]")
    })?;

    Ok((file, tag, exclude_prefix))
}

fn extract_string_attr(attr: &syn::Attribute) -> syn::Result<Option<String>> {
    let meta = &attr.meta;
    if let Meta::List(list) = meta {
        let lit: Lit = list.parse_args()?;
        if let Lit::Str(s) = lit {
            return Ok(Some(s.value()));
        }
    }
    Ok(None)
}

fn get_absolute_path(file: &str) -> anyhow::Result<String> {
    let file_path = std::path::Path::new(file);
    if !file_path.exists() {
        let cwd = std::env::current_dir().unwrap_or_default();
        let absolute = cwd.join(file_path);
        anyhow::bail!("File not found: {} (checked: {})", file, absolute.display());
    }
    let absolute = std::fs::canonicalize(file_path)
        .with_context(|| format!("Failed to canonicalize path: {}", file))?;
    Ok(absolute.to_string_lossy().to_string())
}

fn extract_pixel_locations(
    file: &str,
    tag: &str,
    exclude_prefix: Option<&str>,
) -> anyhow::Result<Vec<(i32, i32)>> {
    let file_stem = std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let cache_dir = format!("assets/_generated/pixcache/{}", file_stem);
    std::fs::create_dir_all(&cache_dir)?;

    let cache_png = format!("{}/{}.png", cache_dir, tag);

    let exclude_prefixes: Vec<String> = exclude_prefix
        .map(|p| vec![p.to_string()])
        .unwrap_or_default();
    aseprite::export_single_frame(file, tag, &cache_png, &exclude_prefixes)?;

    let img = image::open(&cache_png)
        .with_context(|| format!("Failed to open exported PNG: {}", cache_png))?
        .to_rgba8();
    let (width, height) = img.dimensions();

    if width % 2 != 0 || height % 2 != 0 {
        anyhow::bail!("Sprite dimensions must be even, got {}x{}", width, height);
    }

    let center_x = (width / 2) as i32;
    let center_y = (height / 2) as i32;

    let mut locations = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            if pixel[3] > 0 {
                let rel_x = x as i32 - center_x;
                let rel_y = center_y - 1 - y as i32;
                locations.push((rel_x, rel_y));
            }
        }
    }

    Ok(locations)
}

#[proc_macro_derive(Anim, attributes(file, fps, exclude_prefix, next))]
pub fn derive_anim(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    if !is_enum(&input) {
        return syn::Error::new_spanned(&input.ident, "Anim can only be derived for enums")
            .to_compile_error()
            .into();
    }

    let anim_attrs = match parse_anim_struct_attributes(&input) {
        Ok(attrs) => attrs,
        Err(e) => return e.to_compile_error().into(),
    };

    let absolute_file = match get_absolute_path(&anim_attrs.file) {
        Ok(p) => p,
        Err(e) => {
            return syn::Error::new_spanned(&input.ident, format!("{}", e))
                .to_compile_error()
                .into();
        }
    };

    let variants = match get_enum_variants(&input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let variant_infos: Vec<AnimVariantInfo> = match variants
        .iter()
        .map(|v| parse_anim_variant(v, &absolute_file, &anim_attrs))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(infos) => infos,
        Err(e) => return e.to_compile_error().into(),
    };

    let struct_name = &input.ident;
    let struct_snake = to_snake_case(&struct_name.to_string());
    let output_dir = format!("assets/_generated/anim/{}", struct_snake);

    let tags: Vec<String> = variant_infos.iter().map(|v| v.tag.clone()).collect();
    let exclude_prefixes: Vec<String> = anim_attrs
        .exclude_prefix
        .as_ref()
        .map(|p| vec![p.clone()])
        .unwrap_or_default();

    let batch_results = match aseprite::batch_export_sprite_sheets(
        &absolute_file,
        &tags,
        &output_dir,
        &exclude_prefixes,
    ) {
        Ok(r) => r,
        Err(e) => {
            return syn::Error::new_spanned(&input.ident, format!("Export failed: {}", e))
                .to_compile_error()
                .into();
        }
    };

    let mut export_infos: Vec<(String, String, aseprite::AnimExportInfo)> = Vec::new();
    for info in &variant_infos {
        let result = match batch_results.iter().find(|r| r.tag == info.tag) {
            Some(r) => r,
            None => {
                return syn::Error::new_spanned(
                    &input.ident,
                    format!("Missing export for tag '{}'", info.tag),
                )
                .to_compile_error()
                .into();
            }
        };
        export_infos.push((
            info.variant_name.clone(),
            result.output_path.clone(),
            result.info.clone(),
        ));
    }

    let variant_idents: Vec<_> = variant_infos
        .iter()
        .map(|v| syn::Ident::new(&v.variant_name, proc_macro2::Span::call_site()))
        .collect();

    let tags: Vec<_> = variant_infos.iter().map(|v| &v.tag).collect();

    let frame_counts: Vec<_> = export_infos
        .iter()
        .map(|(_, _, info)| info.frame_count)
        .collect();

    let frame_widths: Vec<_> = export_infos
        .iter()
        .map(|(_, _, info)| info.frame_width)
        .collect();

    let frame_heights: Vec<_> = export_infos
        .iter()
        .map(|(_, _, info)| info.frame_height)
        .collect();

    let file_paths: Vec<_> = export_infos
        .iter()
        .map(|(_, path, _)| path.clone())
        .collect();

    let asset_paths: Vec<_> = file_paths
        .iter()
        .map(|path| path.strip_prefix("assets/").unwrap_or(path).to_string())
        .collect();

    let include_bytes_checks: Vec<_> = file_paths
        .iter()
        .map(|path| {
            let abs_path = std::fs::canonicalize(path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.clone());
            quote! { const _: &[u8] = include_bytes!(#abs_path); }
        })
        .collect();

    let variant_indices: Vec<_> = (0..variant_infos.len()).collect();

    let fps_values: Vec<_> = variant_infos
        .iter()
        .map(|info| {
            if let Some(fps) = info.fps {
                let fps_lit = fps as f32;
                quote! { Some(#fps_lit) }
            } else {
                quote! { None }
            }
        })
        .collect();

    let next_indices: Vec<_> = variant_infos
        .iter()
        .enumerate()
        .map(|(i, info)| match &info.next {
            AnimNext::State(next_name) => {
                let next_idx = variant_infos
                    .iter()
                    .position(|v| &v.variant_name == next_name)
                    .unwrap_or(i);
                quote! { bits::bits_ui::anim::AnimNextIndex::Index(#next_idx) }
            }
            AnimNext::Loop => {
                quote! { bits::bits_ui::anim::AnimNextIndex::Index(#i) }
            }
            AnimNext::Remove => {
                quote! { bits::bits_ui::anim::AnimNextIndex::Remove }
            }
            AnimNext::Despawn => {
                quote! { bits::bits_ui::anim::AnimNextIndex::Despawn }
            }
        })
        .collect();

    let table_name = syn::Ident::new(
        &format!("{}_ANIM_TABLE", struct_name.to_string().to_uppercase()),
        proc_macro2::Span::call_site(),
    );

    let default_variant = variant_infos
        .iter()
        .find(|v| v.is_default)
        .unwrap_or(&variant_infos[0]);

    let pixel_locations = match extract_pixel_locations(
        &absolute_file,
        &default_variant.tag,
        anim_attrs.exclude_prefix.as_deref(),
    ) {
        Ok(locs) => locs,
        Err(e) => {
            return syn::Error::new_spanned(
                &input.ident,
                format!(
                    "Failed to extract pixel locations for default variant '{}': {}",
                    default_variant.variant_name, e
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    let pixel_exprs: Vec<_> = pixel_locations
        .iter()
        .map(|(x, y)| quote! { bevy::prelude::IVec2::new(#x, #y) })
        .collect();

    let expanded = quote! {
        #(#include_bytes_checks)*

        static #table_name: &[bits::bits_ui::anim::AnimVariant] = &[
            #(
                bits::bits_ui::anim::AnimVariant {
                    tag: #tags,
                    fps: #fps_values,
                    frame_count: #frame_counts,
                    frame_size: (#frame_widths, #frame_heights),
                    asset_path: #asset_paths,
                    next: #next_indices,
                },
            )*
        ];

        impl bits::bits_ui::anim::Anim for #struct_name {
            fn table() -> &'static [bits::bits_ui::anim::AnimVariant] {
                #table_name
            }

            fn index(&self) -> usize {
                match self {
                    #(Self::#variant_idents => #variant_indices,)*
                }
            }

            fn from_index(index: usize) -> Self {
                match index {
                    #(#variant_indices => Self::#variant_idents,)*
                    _ => Self::default(),
                }
            }
        }

        impl bits::bits_ui::assemble::Assemblable for #struct_name {
            fn get_pixel_locations() -> Vec<bevy::prelude::IVec2> {
                vec![#(#pixel_exprs),*]
            }
        }

        impl #struct_name {
            pub fn plugin(app: &mut bevy::prelude::App) {
                bits::bits_ui::anim::register_anim::<#struct_name>(app);
            }
        }
    };

    TokenStream::from(expanded)
}

fn is_enum(input: &DeriveInput) -> bool {
    matches!(&input.data, Data::Enum(_))
}

struct AnimStructAttrs {
    file: String,
    fps: Option<u32>,
    exclude_prefix: Option<String>,
}

fn parse_anim_struct_attributes(input: &DeriveInput) -> syn::Result<AnimStructAttrs> {
    let mut file: Option<String> = None;
    let mut fps: Option<u32> = None;
    let mut exclude_prefix: Option<String> = None;

    for attr in &input.attrs {
        if attr.path().is_ident("file") {
            if let Some(s) = extract_string_attr(attr)? {
                file = Some(s);
            }
        } else if attr.path().is_ident("fps") {
            if let Some(n) = extract_int_attr(attr)? {
                fps = Some(n);
            }
        } else if attr.path().is_ident("exclude_prefix") {
            if let Some(s) = extract_string_attr(attr)? {
                exclude_prefix = Some(s);
            }
        }
    }

    let file = file.ok_or_else(|| {
        syn::Error::new_spanned(&input.ident, "Missing required attribute: #[file(\"...\")]")
    })?;

    Ok(AnimStructAttrs {
        file,
        fps,
        exclude_prefix,
    })
}

fn extract_int_attr(attr: &syn::Attribute) -> syn::Result<Option<u32>> {
    let meta = &attr.meta;
    if let Meta::List(list) = meta {
        let lit: Lit = list.parse_args()?;
        if let Lit::Int(i) = lit {
            return Ok(Some(i.base10_parse()?));
        }
    }
    Ok(None)
}

fn get_enum_variants(input: &DeriveInput) -> syn::Result<Vec<&Variant>> {
    match &input.data {
        Data::Enum(data) => {
            for variant in &data.variants {
                if !matches!(variant.fields, Fields::Unit) {
                    return Err(syn::Error::new_spanned(
                        variant,
                        "Anim enum variants must be unit variants",
                    ));
                }
            }
            Ok(data.variants.iter().collect())
        }
        _ => Err(syn::Error::new_spanned(input, "Expected enum")),
    }
}

#[derive(Clone)]
enum AnimNext {
    State(String),
    Loop,
    Remove,
    Despawn,
}

struct AnimVariantInfo {
    variant_name: String,
    tag: String,
    fps: Option<u32>,
    next: AnimNext,
    is_default: bool,
}

fn parse_anim_variant(
    variant: &Variant,
    file: &str,
    struct_attrs: &AnimStructAttrs,
) -> syn::Result<AnimVariantInfo> {
    let variant_name = variant.ident.to_string();
    let tag = to_snake_case(&variant_name);

    if let Err(e) = aseprite::validate_tag(file, &tag) {
        return Err(syn::Error::new_spanned(
            variant,
            format!(
                "Tag '{}' (from variant '{}') not found: {}",
                tag, variant_name, e
            ),
        ));
    }

    let mut fps: Option<u32> = struct_attrs.fps;
    let mut next: Option<AnimNext> = None;
    let mut is_default = false;

    for attr in &variant.attrs {
        if attr.path().is_ident("fps") {
            if let Some(n) = extract_int_attr(attr)? {
                fps = Some(n);
            }
        } else if attr.path().is_ident("next") {
            let meta = &attr.meta;
            if let Meta::List(list) = meta {
                let ident: syn::Ident = list.parse_args()?;
                let ident_str = ident.to_string();
                next = Some(match ident_str.as_str() {
                    "ANIM_DESPAWN" | "AnimDespawn" => AnimNext::Despawn,
                    "ANIM_REMOVE" | "AnimRemove" => AnimNext::Remove,
                    _ => AnimNext::State(ident_str),
                });
            }
        } else if attr.path().is_ident("default") {
            is_default = true;
        }
    }

    let next = next.unwrap_or(AnimNext::Loop);

    Ok(AnimVariantInfo {
        variant_name,
        tag,
        fps,
        next,
        is_default,
    })
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}

// ============================================================================
// anim_enum! macro
// ============================================================================

struct AnimEnumInput {
    vis: syn::Visibility,
    name: syn::Ident,
    file: String,
    exclude: Option<String>,
    default: syn::Ident,
    fps: Option<u32>,
    next_overrides: HashMap<String, AnimNext>,
}

impl Parse for AnimEnumInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse: pub enum Name,
        let vis: syn::Visibility = input.parse()?;
        input.parse::<Token![enum]>()?;
        let name: syn::Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let mut file: Option<String> = None;
        let mut exclude: Option<String> = None;
        let mut default: Option<syn::Ident> = None;
        let mut fps: Option<u32> = None;
        let mut next_overrides: HashMap<String, AnimNext> = HashMap::new();

        // Parse key: value pairs
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match key.to_string().as_str() {
                "file" => {
                    let lit: syn::LitStr = input.parse()?;
                    file = Some(lit.value());
                }
                "exclude" => {
                    let lit: syn::LitStr = input.parse()?;
                    exclude = Some(lit.value());
                }
                "default" => {
                    default = Some(input.parse()?);
                }
                "fps" => {
                    let lit: syn::LitInt = input.parse()?;
                    fps = Some(lit.base10_parse()?);
                }
                "next" => {
                    let content;
                    syn::braced!(content in input);
                    while !content.is_empty() {
                        let variant: syn::Ident = content.parse()?;
                        content.parse::<Token![=>]>()?;
                        let target: syn::Ident = content.parse()?;
                        let target_str = target.to_string();
                        let next = match target_str.as_str() {
                            "ANIM_DESPAWN" | "AnimDespawn" => AnimNext::Despawn,
                            "ANIM_REMOVE" | "AnimRemove" => AnimNext::Remove,
                            _ => AnimNext::State(target_str),
                        };
                        next_overrides.insert(variant.to_string(), next);
                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                _ => {
                    return Err(syn::Error::new(key.span(), format!("Unknown key: {}", key)));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let file =
            file.ok_or_else(|| syn::Error::new(input.span(), "Missing required field: file"))?;
        let default = default
            .ok_or_else(|| syn::Error::new(input.span(), "Missing required field: default"))?;

        Ok(AnimEnumInput {
            vis,
            name,
            file,
            exclude,
            default,
            fps,
            next_overrides,
        })
    }
}

#[proc_macro]
pub fn anim_enum(input: TokenStream) -> TokenStream {
    let config = parse_macro_input!(input as AnimEnumInput);

    // Get absolute file path
    let absolute_file = match get_absolute_path(&config.file) {
        Ok(p) => p,
        Err(e) => {
            return syn::Error::new(proc_macro2::Span::call_site(), format!("{}", e))
                .to_compile_error()
                .into();
        }
    };

    // List tags from aseprite file
    let tags = match aseprite::list_tags(&absolute_file) {
        Ok(t) => t,
        Err(e) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Failed to list tags: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    // Filter by exclude prefix and convert to variant info
    let variant_infos: Vec<AnimVariantInfo> = tags
        .iter()
        .filter(|tag| {
            if let Some(ref prefix) = config.exclude {
                !tag.starts_with(prefix)
            } else {
                true
            }
        })
        .map(|tag| {
            let variant_name = to_pascal_case(tag);
            let is_default = variant_name == config.default.to_string();
            let next = config
                .next_overrides
                .get(&variant_name)
                .cloned()
                .unwrap_or(AnimNext::Loop);
            AnimVariantInfo {
                variant_name,
                tag: tag.clone(),
                fps: config.fps,
                next,
                is_default,
            }
        })
        .collect();

    if variant_infos.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "No variants found after filtering",
        )
        .to_compile_error()
        .into();
    }

    // Validate default exists
    if !variant_infos.iter().any(|v| v.is_default) {
        return syn::Error::new(
            config.default.span(),
            format!("Default variant '{}' not found in tags", config.default),
        )
        .to_compile_error()
        .into();
    }

    // Generate the implementation
    match generate_anim_impl(
        &config.name,
        &config.vis,
        &absolute_file,
        config.exclude.as_deref(),
        &variant_infos,
        true, // generate_enum = true for anim_enum!
    ) {
        Ok(tokens) => tokens,
        Err(e) => syn::Error::new(proc_macro2::Span::call_site(), format!("{}", e))
            .to_compile_error()
            .into(),
    }
}

/// Shared generation logic for both derive_anim and anim_enum!
fn generate_anim_impl(
    struct_name: &syn::Ident,
    vis: &syn::Visibility,
    absolute_file: &str,
    exclude_prefix: Option<&str>,
    variant_infos: &[AnimVariantInfo],
    generate_enum: bool,
) -> anyhow::Result<TokenStream> {
    let struct_snake = to_snake_case(&struct_name.to_string());
    let output_dir = format!("assets/_generated/anim/{}", struct_snake);

    let tags: Vec<String> = variant_infos.iter().map(|v| v.tag.clone()).collect();
    let exclude_prefixes: Vec<String> = exclude_prefix
        .map(|p| vec![p.to_string()])
        .unwrap_or_default();

    let batch_results = aseprite::batch_export_sprite_sheets(
        absolute_file,
        &tags,
        &output_dir,
        &exclude_prefixes,
    )?;

    let mut export_infos: Vec<(String, String, aseprite::AnimExportInfo)> = Vec::new();
    for info in variant_infos {
        let result = batch_results
            .iter()
            .find(|r| r.tag == info.tag)
            .ok_or_else(|| anyhow::anyhow!("Missing export for tag '{}'", info.tag))?;
        export_infos.push((
            info.variant_name.clone(),
            result.output_path.clone(),
            result.info.clone(),
        ));
    }

    let variant_idents: Vec<_> = variant_infos
        .iter()
        .map(|v| syn::Ident::new(&v.variant_name, proc_macro2::Span::call_site()))
        .collect();

    let tags: Vec<_> = variant_infos.iter().map(|v| &v.tag).collect();

    let frame_counts: Vec<_> = export_infos
        .iter()
        .map(|(_, _, info)| info.frame_count)
        .collect();

    let frame_widths: Vec<_> = export_infos
        .iter()
        .map(|(_, _, info)| info.frame_width)
        .collect();

    let frame_heights: Vec<_> = export_infos
        .iter()
        .map(|(_, _, info)| info.frame_height)
        .collect();

    let file_paths: Vec<_> = export_infos
        .iter()
        .map(|(_, path, _)| path.clone())
        .collect();

    let asset_paths: Vec<_> = file_paths
        .iter()
        .map(|path| path.strip_prefix("assets/").unwrap_or(path).to_string())
        .collect();

    let include_bytes_checks: Vec<_> = file_paths
        .iter()
        .map(|path| {
            let abs_path = std::fs::canonicalize(path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.clone());
            quote! { const _: &[u8] = include_bytes!(#abs_path); }
        })
        .collect();

    let variant_indices: Vec<_> = (0..variant_infos.len()).collect();

    let fps_values: Vec<_> = variant_infos
        .iter()
        .map(|info| {
            if let Some(fps) = info.fps {
                let fps_lit = fps as f32;
                quote! { Some(#fps_lit) }
            } else {
                quote! { None }
            }
        })
        .collect();

    let next_indices: Vec<_> = variant_infos
        .iter()
        .enumerate()
        .map(|(i, info)| match &info.next {
            AnimNext::State(next_name) => {
                let next_idx = variant_infos
                    .iter()
                    .position(|v| &v.variant_name == next_name)
                    .unwrap_or(i);
                quote! { bits::bits_ui::anim::AnimNextIndex::Index(#next_idx) }
            }
            AnimNext::Loop => {
                quote! { bits::bits_ui::anim::AnimNextIndex::Index(#i) }
            }
            AnimNext::Remove => {
                quote! { bits::bits_ui::anim::AnimNextIndex::Remove }
            }
            AnimNext::Despawn => {
                quote! { bits::bits_ui::anim::AnimNextIndex::Despawn }
            }
        })
        .collect();

    let table_name = syn::Ident::new(
        &format!("{}_ANIM_TABLE", struct_name.to_string().to_uppercase()),
        proc_macro2::Span::call_site(),
    );

    let default_variant = variant_infos
        .iter()
        .find(|v| v.is_default)
        .unwrap_or(&variant_infos[0]);

    let pixel_locations = extract_pixel_locations(
        absolute_file,
        &default_variant.tag,
        exclude_prefix,
    )
    .with_context(|| {
        format!(
            "Failed to extract pixel locations for default variant '{}'",
            default_variant.variant_name
        )
    })?;

    let pixel_exprs: Vec<_> = pixel_locations
        .iter()
        .map(|(x, y)| quote! { bevy::prelude::IVec2::new(#x, #y) })
        .collect();

    let default_ident = syn::Ident::new(
        &default_variant.variant_name,
        proc_macro2::Span::call_site(),
    );

    // Generate enum definition if requested (for anim_enum! macro)
    let enum_def = if generate_enum {
        quote! {
            #[derive(Clone, Copy, Debug)]
            #vis enum #struct_name {
                #(#variant_idents,)*
            }

            impl Default for #struct_name {
                fn default() -> Self {
                    Self::#default_ident
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #enum_def

        #(#include_bytes_checks)*

        static #table_name: &[bits::bits_ui::anim::AnimVariant] = &[
            #(
                bits::bits_ui::anim::AnimVariant {
                    tag: #tags,
                    fps: #fps_values,
                    frame_count: #frame_counts,
                    frame_size: (#frame_widths, #frame_heights),
                    asset_path: #asset_paths,
                    next: #next_indices,
                },
            )*
        ];

        impl bits::bits_ui::anim::Anim for #struct_name {
            fn table() -> &'static [bits::bits_ui::anim::AnimVariant] {
                #table_name
            }

            fn index(&self) -> usize {
                match self {
                    #(Self::#variant_idents => #variant_indices,)*
                }
            }

            fn from_index(index: usize) -> Self {
                match index {
                    #(#variant_indices => Self::#variant_idents,)*
                    _ => Self::default(),
                }
            }
        }

        impl bits::bits_ui::assemble::Assemblable for #struct_name {
            fn get_pixel_locations() -> Vec<bevy::prelude::IVec2> {
                vec![#(#pixel_exprs),*]
            }
        }

        impl #struct_name {
            pub fn plugin(app: &mut bevy::prelude::App) {
                bits::bits_ui::anim::register_anim::<#struct_name>(app);
            }
        }
    };

    Ok(TokenStream::from(expanded))
}
