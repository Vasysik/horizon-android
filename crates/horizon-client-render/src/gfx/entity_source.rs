//! Equivalent GLSL ES lowering for legacy entity-program linkers.

/// Adreno 3xx's linker may call exit(1) for the dynamic white-overlay table
/// lookup in vs_cloud. Keep all sixteen exact OverlayTexture texels, but use
/// constant indices. The caller applies this only to GLES; Vulkan is untouched.
pub(super) fn legacy_overlay(source: &str) -> String {
    let mut helper = String::from("fn legacy_white_overlay(u: u32) -> f32 {\n");
    for i in 0..15 {
        helper.push_str(&format!(
            "    if u == {i}u {{ return WHITE_OVERLAY_ALPHA[{i}]; }}\n"
        ));
    }
    helper.push_str("    return WHITE_OVERLAY_ALPHA[15];\n}\n");
    source
        .replacen("fn overlay_texel(", &(helper + "fn overlay_texel("), 1)
        .replace(
            "WHITE_OVERLAY_ALPHA[overlay - 16u]",
            "legacy_white_overlay(overlay - 16u)",
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_entity_entrypoint_retains_the_exact_white_overlay_table() {
        let original = crate::overlay::wgsl_constants() + include_str!("../entity.wgsl");
        let lowered = super::super::terrain_source::gles_uniforms(&legacy_overlay(&original));
        assert!(!lowered.contains("WHITE_OVERLAY_ALPHA[overlay - 16u]"));
        assert!(lowered.contains("legacy_white_overlay(overlay - 16u)"));
        for i in 0..16 {
            assert!(lowered.contains(&format!("return WHITE_OVERLAY_ALPHA[{i}];")));
        }
        let module = naga::front::wgsl::parse_str(&lowered).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        assert_eq!(
            module.entry_points.len(),
            naga::front::wgsl::parse_str(&original)
                .unwrap()
                .entry_points
                .len()
        );
    }
}

#[cfg(test)]
#[path = "entity_source/gpu_tests.rs"]
mod gpu_tests;
