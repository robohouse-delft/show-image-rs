#version 420
// vi: ft=glsl

out gl_PerVertex {
	vec4 gl_Position;
};

layout(location = 0) out vec2 texture_coords;

layout(set = 0, binding = 0) uniform WindowUniforms {
	vec2 scale;
};

const vec2 POSITIONS[6] = vec2[6](
	vec2(-1.0,  1.0),
	vec2( 1.0,  1.0),
	vec2( 1.0, -1.0),
	vec2(-1.0,  1.0),
	vec2( 1.0, -1.0),
	vec2(-1.0, -1.0)
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
	gl_Position = vec4(POSITIONS[gl_VertexIndex] * scale, 0.0, 1.0);
	texture_coords = TEXTURE_POSITIONS[gl_VertexIndex];
}
