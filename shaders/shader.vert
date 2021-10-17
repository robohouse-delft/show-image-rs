#version 420
// vi: ft=glsl

out gl_PerVertex {
	vec4 gl_Position;
};

layout(location = 0) out vec2 texture_coords;

layout(set = 0, binding = 0) uniform WindowUniforms {
	vec2 image_size;
	mat3 transform;
};

const vec2 POSITIONS[6] = vec2[6](
	vec2(0.0, 0.0),
	vec2(1.0, 0.0),
	vec2(1.0, 1.0),
	vec2(0.0, 0.0),
	vec2(1.0, 1.0),
	vec2(0.0, 1.0)
);

// Flip screen space coordinates to put the origin at the top left corner,
// and have the positive Y axis pointing down.
const mat3 flip_y = mat3(vec3(1.0, 0.0, 0.0), vec3(0.0, -1.0, 0.0), vec3(0.0, 1.0, 1.0));

void main() {
	vec2 position = (flip_y * transform * vec3(POSITIONS[gl_VertexIndex], 1.0)).xy;

	// Adjust for weird screen space going from -1.0 to 1.0 instead of 0.0 to 1.0.
	position = 2.0 * position - vec2(1.0, 1.0);

	gl_Position = vec4(position, 0.0, 1.0);
	texture_coords = (image_size) * POSITIONS[gl_VertexIndex];
}
