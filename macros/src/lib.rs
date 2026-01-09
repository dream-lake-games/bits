extern crate proc_macro;

mod aseprite;

use anyhow::Context;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Lit, Meta, Variant, parse_macro_input};

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
    aseprite::validate_tag(file, tag)?;

    let temp_dir = std::env::temp_dir();
    let temp_png = temp_dir.join(format!(
        "assemblable_{}_{}.png",
        file.replace(['/', '\\', '.'], "_"),
        tag
    ));

    let mut builder = aseprite::ExportBuilder::new(file, tag);
    if let Some(prefix) = exclude_prefix {
        builder = builder.exclude_prefix(prefix);
    }
    builder
        .export_to_file(temp_png.to_str().unwrap())
        .with_context(|| format!("Failed to export aseprite file '{}' tag '{}'", file, tag))?;

    let img = image::open(&temp_png)
        .with_context(|| format!("Failed to open exported PNG: {}", temp_png.display()))?
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

    let _ = std::fs::remove_file(&temp_png);

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
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        return syn::Error::new_spanned(
            &input.ident,
            format!("Failed to create output directory '{}': {}", output_dir, e),
        )
        .to_compile_error()
        .into();
    }

    let mut export_infos: Vec<(String, String, aseprite::AnimExportInfo)> = Vec::new();
    for info in &variant_infos {
        let output_path = format!("{}/{}.png", output_dir, info.tag);
        let mut builder = aseprite::ExportBuilder::new(&absolute_file, &info.tag);
        if let Some(ref prefix) = anim_attrs.exclude_prefix {
            builder = builder.exclude_prefix(prefix);
        }
        match builder.export_sprite_sheet(&output_path) {
            Ok(export_info) => {
                export_infos.push((info.variant_name.clone(), output_path, export_info));
            }
            Err(e) => {
                return syn::Error::new_spanned(
                    &input.ident,
                    format!("Failed to export tag '{}': {}", info.tag, e),
                )
                .to_compile_error()
                .into();
            }
        }
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
