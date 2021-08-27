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

layout(set = 1, binding = 1) buffer readonly Data {
	uint data[];
};

uint extract_u8(uint i) {
	uint word = data[i / 4];
	uint offset = (i % 4) * 8;
	return word >> offset & 0xFF;
}

float extract_unorm8(uint i) {
	return float(extract_u8(i)) / 255.0;
}

vec4 get_pixel(uint x, uint y) {
	uint i = x * stride_x + y * stride_y;

	// Mono8
	if (format == 0) {
		float mono = extract_unorm8(i);
		return vec4(mono, mono, mono, 1.0);

	// MonoAlpha8(Unpremultiplied)
	} else if (format == 1) {
		float mono = extract_unorm8(i);
		float a    = extract_unorm8(i + 1);
		return vec4(mono, mono, mono, a);

	// MonoAlpha8(Premultiplied)
	} else if (format == 2) {
		float a    = float(extract_u8(i + 1));
		float mono = float(extract_u8(i)) / a;
		return vec4(mono, mono, mono, a);

	// Bgr8
	} else if (format == 3) {
		float b = extract_unorm8(i + 0);
		float g = extract_unorm8(i + 1);
		float r = extract_unorm8(i + 2);
		return vec4(r, g, b, 1.0);

	// Bgra8(Unpremultiplied)
	} else if (format == 4) {
		float b = extract_unorm8(i + 0);
		float g = extract_unorm8(i + 1);
		float r = extract_unorm8(i + 2);
		float a = extract_unorm8(i + 3);
		return vec4(r, g, b, a);

	// Bgra8(Premultiplied)
	} else if (format == 5) {
		float a = float(extract_u8(i + 3));
		float b = float(extract_u8(i + 0)) / a;
		float g = float(extract_u8(i + 1)) / a;
		float r = float(extract_u8(i + 2)) / a;
		return vec4(r, g, b, a / 255.0);

	// Rgb8
	} else if (format == 6) {
		float r = extract_unorm8(i + 0);
		float g = extract_unorm8(i + 1);
		float b = extract_unorm8(i + 2);
		return vec4(r, g, b, 1.0);

	// Rgba8(Unpremultiplied)
	} else if (format == 7) {
		float r = extract_unorm8(i + 0);
		float g = extract_unorm8(i + 1);
		float b = extract_unorm8(i + 2);
		float a = extract_unorm8(i + 3);
		return vec4(r, g, b, a);

	// Rgba8(Premultiplied)
	} else if (format == 8) {
		float a = float(extract_u8(i + 3));
		float r = float(extract_u8(i + 0)) / a;
		float g = float(extract_u8(i + 1)) / a;
		float b = float(extract_u8(i + 2)) / a;
		return vec4(r, g, b, a / 255.0);

	} else {
		return vec4(1.0, 0.0, 1.0, 1.0);
	}
}

void main() {
	uint x = uint(floor(texture_coords.x));
	uint y = uint(floor(texture_coords.y));
	if (x >= width || y >= height) {
		out_color = vec4(0.0, 0.0, 0.0, 0.0);
	} else {
		out_color = get_pixel(x, y);
	}
}
