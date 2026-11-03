// CubeMap's position-only cube and panorama.vsh/fsh: sample its untransformed direction.
struct Projection { view_proj: mat4x4<f32> }
@group(0) @binding(0) var<uniform> projection: Projection;
@group(0) @binding(1) var panorama: texture_cube<f32>;
@group(0) @binding(2) var panorama_sampler: sampler;
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) direction: vec3<f32>,
}
@vertex fn vs_panorama(@location(0) position: vec3<f32>) -> Out {
    var out: Out;
    out.clip = projection.view_proj * vec4(position, 1.0);
    out.direction = position;
    return out;
}
@fragment fn fs_panorama(in: Out) -> @location(0) vec4<f32> {
    return textureSample(panorama, panorama_sampler, in.direction);
}
