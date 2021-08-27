#version 430
// vi: ft=glsl

layout(location = 0) in vec2 texture_coords;
layout(location = 0) out uvec4 out_color;

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

uvec4 get_pixel(uint x, uint y) {
	uint i = x * stride_x + y * stride_y;

	// Mono8
	if (format == 0) {
		float mono = extract_u8(i);
		return uvec4(mono, mono, mono, 255);

	// MonoAlpha8(Unpremultiplied)
	} else if (format == 1) {
		uint mono = extract_u8(i);
		uint a    = extract_u8(i + 1);
		return uvec4(mono, mono, mono, a);

	// MonoAlpha8(Premultiplied)
	} else if (format == 2) {
		uint a    = extract_u8(i + 1);
		uint mono = extract_u8(i) * 255 / a;
		return uvec4(mono, mono, mono, a);

	// Bgr8
	} else if (format == 3) {
		uint b = extract_u8(i + 0);
		uint g = extract_u8(i + 1);
		uint r = extract_u8(i + 2);
		return uvec4(r, g, b, 255);

	// Bgra8(Unpremultiplied)
	} else if (format == 4) {
		uint b = extract_u8(i + 0);
		uint g = extract_u8(i + 1);
		uint r = extract_u8(i + 2);
		uint a = extract_u8(i + 3);
		return uvec4(r, g, b, a);

	// Bgra8(Premultiplied)
	} else if (format == 5) {
		uint a = extract_u8(i + 3);
		uint b = extract_u8(i + 0) * 255 / a;
		uint g = extract_u8(i + 1) * 255 / a;
		uint r = extract_u8(i + 2) * 255 / a;
		return uvec4(r, g, b, a);

	// Rgb8
	} else if (format == 6) {
		uint r = extract_u8(i + 0);
		uint g = extract_u8(i + 1);
		uint b = extract_u8(i + 2);
		return uvec4(r, g, b, 255);

	// Rgba8(Unpremultiplied)
	} else if (format == 7) {
		uint r = extract_u8(i + 0);
		uint g = extract_u8(i + 1);
		uint b = extract_u8(i + 2);
		uint a = extract_u8(i + 3);
		return uvec4(r, g, b, a);

	// Rgba8(Premultiplied)
	} else if (format == 8) {
		uint a = extract_u8(i + 3);
		uint r = extract_u8(i + 0) * 255 / a;
		uint g = extract_u8(i + 1) * 255 / a;
		uint b = extract_u8(i + 2) * 255 / a;
		return uvec4(r, g, b, a);

	} else {
		return uvec4(255, 0, 255, 255);
	}
}

void main() {
	uint x = uint(floor(texture_coords.x));
	uint y = uint(floor(texture_coords.y));
	if (x >= width || y >= height) {
		out_color = uvec4(0, 0, 0, 0);
	} else {
		out_color = get_pixel(x, y);
	}
}
