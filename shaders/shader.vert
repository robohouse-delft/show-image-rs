#version 420
// vi: ft=glsl

out gl_PerVertex {
	vec4 gl_Position;
};

layout(location = 0) out vec2 texture_coords;

layout(set = 0, binding = 0) uniform WindowUniforms {
	vec2 offset;
	vec2 relative_size;
	vec2 pixel_size;
};

const vec2 POSITIONS[6] = vec2[6](
	vec2(0.0, 1.0),
	vec2(1.0, 1.0),
	vec2(1.0, 0.0),
	vec2(0.0, 1.0),
	vec2(1.0, 0.0),
	vec2(0.0, 0.0)
);

const vec2 TEXTURE_POSITIONS[6] = vec2[6](
	vec2(0.0, 0.0),
	vec2(1.0, 0.0),
	vec2(1.0, 1.0),
	vec2(0.0, 0.0),
	vec2(1.0, 1.0),
	vec2(0.0, 1.0)
);

void main() {
	vec2 position = offset + relative_size * POSITIONS[gl_VertexIndex];
	position = 2.0 * position - vec2(1.0, 1.0);
	gl_Position = vec4(position, 0.0, 1.0);
	texture_coords = (pixel_size - vec2(1.0, 1.0)) * TEXTURE_POSITIONS[gl_VertexIndex];
}
