#version 450

layout(location = 0) in vec2 texture_coords;
layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform texture2D diffuse_texture;
layout(set = 0, binding = 1) uniform sampler   diffuse_sampler;

void main() {
    out_color = texture(sampler2D(diffuse_texture, diffuse_sampler), texture_coords);
}
