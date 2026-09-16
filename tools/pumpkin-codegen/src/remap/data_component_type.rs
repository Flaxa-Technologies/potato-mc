use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use crate::remap::{MappingNode, ParsedMappings, Remapper};
use crate::remap_nodes;
use crate::version::JavaMinecraftVersion;

/// Generates the `TokenStream` for per-version data component type ID remap tables and the
/// `remap_data_component_type_id_for_version`/`remap_data_component_type_id_from_version` functions.
pub fn build() -> TokenStream {
    let remapper: Remapper<_, Option<Vec<u32>>> = Remapper {
        version: JavaMinecraftVersion::V_26_2,
        remapper: |first, second| match (first, second) {
            (Some(first), Some(second)) => Some(
                first
                    .iter()
                    .map(|&id| second.get(id as usize).copied().unwrap_or(id))
                    .collect(),
            ),
            (None, Some(second)) => Some(second.clone()),
            (Some(first), None) => Some(first.clone()),
            (None, None) => None,
        },
        serializer: |&file| {
            ParsedMappings::parse_mapping_file(file, "data_component_type")
                .map(|mappings| mappings.to_u32(file))
        },
    };

    let all_mappings = remap_nodes!(remapper);
    let mapping_size = all_mappings
        .iter()
        .flat_map(|(_, mapping)| {
            mapping
                .as_ref()
                .map(|m| m.iter().copied().max().unwrap_or(0))
        })
        .max()
        .unwrap_or(0) as usize
        + 1;

    let mut static_values = TokenStream::new();
    let mut match_arms_id_for_ver = TokenStream::new();
    let mut match_arms_id_from_ver = TokenStream::new();

    for (ver, mapping) in &all_mappings {
        let Some(mapping) = mapping else {
            continue;
        };
        let versions = crate::remap::version_patterns(*ver);

        // Forward: 26.2 → old version
        {
            let ident = format_ident!(
                "{}",
                format!(
                    "DATA_COMPONENT_TYPE_ID_REMAP_{:?}_TO_{:?}",
                    remapper.version, ver
                )
                .to_uppercase()
            );
            let mapping_tokens: Vec<_> = mapping
                .iter()
                .copied()
                .map(Literal::u32_unsuffixed)
                .collect();
            static_values.extend(quote! {
                pub static #ident: &[u32] = &[#(#mapping_tokens),*];
            });
            match_arms_id_for_ver.extend(quote! {
                #(#versions)|* => #ident
                    .get(data_component_type_id as usize)
                    .copied()
                    .unwrap_or(data_component_type_id),
            });
        }
        // Reverse: old version → 26.2
        {
            let reversed = reverse_mapping(mapping, mapping_size);
            let ident = format_ident!(
                "{}",
                format!(
                    "DATA_COMPONENT_TYPE_ID_REMAP_{:?}_TO_{:?}",
                    ver, remapper.version
                )
                .to_uppercase()
            );
            let mapping_tokens: Vec<_> =
                reversed.into_iter().map(Literal::u32_unsuffixed).collect();
            static_values.extend(quote! {
                pub static #ident: &[u32] = &[#(#mapping_tokens),*];
            });
            match_arms_id_from_ver.extend(quote! {
                #(#versions)|* => #ident
                    .get(data_component_type_id as usize)
                    .copied()
                    .unwrap_or(data_component_type_id),
            });
        }
    }

    let remap_internal_to_26_3: &[u32] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 42, 45, 46, 47, 0, 48,
        49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70,
        71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 88, 89, 90, 91, 92, 93, 94, 95, 96,
        97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114,
        115, 116, 84, 85, 86, 87, 40, 41, 43, 44, 117, 118, 119, 120, 121,
    ];
    let remap_26_3_to_internal: &[u32] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 115, 116, 41, 117, 118, 42,
        43, 44, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65,
        66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 111, 112, 113, 114, 82,
        83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103,
        104, 105, 106, 107, 108, 109, 110, 119, 120, 121, 122, 123,
    ];

    static_values.extend(quote! {
        pub static DATA_COMPONENT_TYPE_ID_REMAP_INTERNAL_TO_V_26_3: &[u32] = &[#(#remap_internal_to_26_3),*];
        pub static DATA_COMPONENT_TYPE_ID_REMAP_V_26_3_TO_INTERNAL: &[u32] = &[#(#remap_26_3_to_internal),*];
    });

    match_arms_id_for_ver.extend(quote! {
        pumpkin_util::version::JavaMinecraftVersion::V_26_3 => DATA_COMPONENT_TYPE_ID_REMAP_INTERNAL_TO_V_26_3
            .get(data_component_type_id as usize)
            .copied()
            .unwrap_or(data_component_type_id),
    });

    match_arms_id_from_ver.extend(quote! {
        pumpkin_util::version::JavaMinecraftVersion::V_26_3 => DATA_COMPONENT_TYPE_ID_REMAP_V_26_3_TO_INTERNAL
            .get(data_component_type_id as usize)
            .copied()
            .unwrap_or(data_component_type_id),
    });

    quote! {
        use pumpkin_util::version::JavaMinecraftVersion;

        #static_values

        #[must_use]
        pub fn remap_data_component_type_id_for_version(
            data_component_type_id: u32,
            version: JavaMinecraftVersion,
        ) -> u32 {
            match version {
                #match_arms_id_for_ver
                _ => data_component_type_id,
            }
        }

        #[must_use]
        pub fn remap_data_component_type_id_from_version(
            data_component_type_id: u32,
            version: JavaMinecraftVersion,
        ) -> u32 {
            match version {
                #match_arms_id_from_ver
                _ => data_component_type_id,
            }
        }
    }
}

fn reverse_mapping(mapping: &[u32], mapped_size: usize) -> Vec<u32> {
    let mut result = vec![0u32; mapped_size];
    for (new_id, old_id) in mapping.iter().enumerate() {
        let old_id = *old_id as usize;
        if old_id != 0 && old_id < mapped_size {
            result[old_id] = new_id as u32;
        }
    }
    result
}
