#version 430
// vi: ft=glsl

layout(location = 0) in vec2 texture_coords;
layout(location = 0) out vec4 out_color;

layout(set = 1, binding = 0) uniform InfoBlock {
	uint format;
	uint width;
	uint height;
	uint stride_x;
	uint stride_y;
};

layout(set = 1, binding = 1) buffer Data {
	uint data[];
};

float extract_unorm8(uint i) {
	uint word = data[i / 4];
	uint offset = (i % 4) * 8;
	return float(word >> offset & 0xFF) / 255.0;
}

vec4 get_pixel(uint x, uint y) {
	uint i = x * stride_x + y * stride_y;

	// Mono8
	if (format == 0) {
		float mono = pow(extract_unorm8(i), 2.2);
		return vec4(mono, mono, mono, 1.0);

	// BGR8
	} else if (format == 1) {
		float b = pow(extract_unorm8(i + 0), 2.2);
		float g = pow(extract_unorm8(i + 1), 2.2);
		float r = pow(extract_unorm8(i + 2), 2.2);
		return vec4(r, g, b, 1.0);

	// BGRA8
	} else if (format == 2) {
		float b = pow(extract_unorm8(i + 0), 2.2);
		float g = pow(extract_unorm8(i + 1), 2.2);
		float r = pow(extract_unorm8(i + 2), 2.2);
		float a = extract_unorm8(i + 3);
		return vec4(r, g, b, a);

	// RGB8
	} else if (format == 3) {
		float r = pow(extract_unorm8(i + 0), 2.2);
		float g = pow(extract_unorm8(i + 1), 2.2);
		float b = pow(extract_unorm8(i + 2), 2.2);
		return vec4(r, g, b, 1.0);

	// RGBA8
	} else if (format == 4) {
		float r = pow(extract_unorm8(i + 0), 2.2);
		float g = pow(extract_unorm8(i + 1), 2.2);
		float b = pow(extract_unorm8(i + 2), 2.2);
		float a = extract_unorm8(i + 3);
		return vec4(r, g, b, a);
	} else {
		return vec4(1.0, 0.0, 1.0, 1.0);
	}
}

void main() {
	uint x = uint(round(texture_coords.x * float(width)));
	uint y = uint(round(texture_coords.y * float(height)));
	out_color = get_pixel(x, y);
}
