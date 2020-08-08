#version 450

out gl_PerVertex {
    vec4 gl_Position;
};

layout(location = 0) out vec2 texture_coords;

void main() {
    uint i = gl_VertexIndex;
    float x = float(i >= 2 && i <= 4);
    float y = float(i & 1);
    gl_Position = vec4(vec2(x, y) * 2.0 - 1.0, 0.0, 1.0);
    texture_coords = vec2(x, 1.0 - y);
}
