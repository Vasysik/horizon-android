//! Legacy GLES lowering. Vulkan retains the original WGSL byte for byte.
use std::borrow::Cow;

pub(super) fn terrain_shader(gl: bool) -> Cow<'static, str> {
    let source = include_str!("../terrain.wgsl");
    if gl {
        Cow::Owned(gles_source(source))
    } else {
        Cow::Borrowed(source)
    }
}

/// GLES 3.0 cannot declare arrays of arrays. Two adjacent vec4 entries
/// retain the original uniform byte layout and every colored-light value.
pub(super) fn gles_uniforms(source: &str) -> String {
    source
        .replace("array<array<vec4<f32>, 2>, 16>", "array<vec4<f32>, 32>")
        .replace(
            "globals.colored_lights[i][0]",
            "globals.colored_lights[2u * i]",
        )
        .replace(
            "globals.colored_lights[i][1]",
            "globals.colored_lights[2u * i + 1u]",
        )
}

fn gles_source(source: &str) -> String {
    let source = gles_uniforms(source).replace("\r\n", "\n");
    let source = unroll_emissive_lookup(source);
    let start = source
        .find("fn sprite_grad(")
        .expect("terrain sprite function");
    let level = source
        .find("fn sprite_level(")
        .expect("terrain level function");
    let end = level + source[level..].find("\n}\n").expect("terrain level end") + 3;
    let rgss = source
        .find("fn sample_rgss(")
        .expect("terrain RGSS function");
    let rgss_end = rgss + source[rgss..].find("\n}\n").expect("terrain RGSS end") + 3;
    let mut output = String::with_capacity(source.len());
    output.push_str(&source[..start]);
    output.push_str(include_str!("terrain_gles.wgsl.inc"));
    output.push_str(&source[end..rgss]);
    output.push_str(&source[rgss_end..]);

    // Adreno 304 renders broken terrain with the separated scalar varyings.
    // Pack values sharing their interpolation qualifier; leave the vertex
    // buffers, texture sampling, fade, lighting, fog and HDR arithmetic alone.
    let start = output
        .find("struct TerrainOut {")
        .expect("terrain interface");
    let end = output[start..]
        .find("struct SkyOut {")
        .expect("sky interface")
        + start;
    let fields_end = output[start..]
        .find("\n}\n")
        .expect("terrain interface end")
        + start
        + 3;
    let mut body = output[fields_end..end].to_owned();
    for (member, packed) in [
        ("emissive_layer", "codes.w"),
        ("layer", "codes.x"),
        ("emission", "codes.y"),
        ("fade", "codes.z"),
        ("spherical", "distances.x"),
        ("cylindrical", "distances.y"),
        ("visibility", "distances.z"),
        ("height", "distances.w"),
    ] {
        for variable in ["in", "out"] {
            body = body.replace(
                &format!("{variable}.{member}"),
                &format!("{variable}.{packed}"),
            );
        }
    }
    output.replace_range(start..end, &(PACKED_INTERFACE.to_owned() + &body));
    output
}

// Adreno 710's compiler fails to link this dynamic uniform-array search in a
// robust EGL context. Preserve the first match, every entry and the no-match
// sentinel; constant-index branches also keep robust access enabled.
fn unroll_emissive_lookup(mut source: String) -> String {
    let start = source
        .find("    for (var i = 0u; i < min(globals.emissive_count.x, 32u); i++) {")
        .expect("terrain emissive lookup");
    let body = source[start..].find('{').unwrap() + start;
    let mut depth = 1;
    let mut end = body + 1;
    while depth != 0 {
        match source.as_bytes()[end] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        end += 1;
    }
    let mut branches = String::new();
    for i in 0..32 {
        let branch = if i == 0 { "if" } else { "else if" };
        branches.push_str(&format!(
            "    {branch} globals.emissive_count.x > {i}u && globals.emissive_layers[{i}].x == (layer & 0x7fffu) {{\n        out.emissive_layer = globals.emissive_layers[{i}].y;\n    }}\n"
        ));
    }
    source.replace_range(start..end, &branches);
    source
}

const PACKED_INTERFACE: &str = "struct TerrainOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) codes: vec4<u32>,
    @location(2) color: vec4<f32>,
    @location(3) distances: vec4<f32>,
    @location(4) local: vec3<f32>,
}
";

#[cfg(test)]
#[path = "terrain_source/hdr_gpu_tests.rs"]
mod hdr_gpu_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vulkan_source_is_unchanged_and_gles_validates_every_entrypoint() {
        assert!(matches!(terrain_shader(false), Cow::Borrowed(_)));
        assert_eq!(terrain_shader(false), include_str!("../terrain.wgsl"));
        let source = terrain_shader(true);
        let original = include_str!("../terrain.wgsl").replace("\r\n", "\n");
        assert_eq!(
            &source[source.find("struct SkyOut {").unwrap()..],
            &original[original.find("struct SkyOut {").unwrap()..],
            "sky and outline shader bodies must remain unchanged"
        );
        assert!(!source.contains("array<array<"));
        assert!(!source.contains("switch layer / SPRITE_SPLIT"));
        assert!(!source.contains("uv + offsets[i] * pixel"));
        for index in 0..7 {
            assert!(source.contains(&format!("array_index == {index}u")));
        }
        assert!(source.contains("let whole = in.codes.y > 4u"));
        assert!(source.contains("if in.codes.w != 0xffffffffu"));
        assert!(!source.contains("in.height"));
        assert!(!source.contains("out.emissive_layer"));
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        let vertex = module
            .entry_points
            .iter()
            .find(|e| e.name == "vs_terrain")
            .unwrap();
        let naga::TypeInner::Struct { members, .. } =
            &module.types[vertex.function.result.as_ref().unwrap().ty].inner
        else {
            panic!("terrain result must be a struct")
        };
        assert_eq!(
            members
                .iter()
                .filter(|m| matches!(m.binding, Some(naga::Binding::Location { .. })))
                .count(),
            5
        );
    }

    #[test]
    fn windows_line_endings_produce_the_same_gles_shader() {
        let source = include_str!("../terrain.wgsl").replace("\r\n", "\n");
        assert_eq!(
            gles_source(&source),
            gles_source(&source.replace('\n', "\r\n"))
        );
    }
}
