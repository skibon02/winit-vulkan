#version 450 core

// substituted per-instance attributes
layout (location = 0) in vec4 in_color;
layout (location = 1) in vec2 in_position;
layout (location = 2) in uint in_trig_time;


// pass to fragment shader
layout(location = 0) out vec2 frag_pos;
layout(location = 1) out vec4 frag_color;

// substituted uniforms definitions
layout (std140, binding = 0) uniform MapStats {
    float r;
    float ar;
} u_map_stats;

// substituted uniforms definitions
layout (std140, binding = 1) uniform Time {
    uint time;
} u_time;

void main() {
    // Triangle strip vertex offsets for each of the four vertices
    vec2 offsets[4] = vec2[](
        vec2(-1.0, -1.0),
        vec2( 1.0, -1.0),
        vec2(-1.0,  1.0),
        vec2( 1.0,  1.0)
    );

    int vertexID = gl_VertexIndex % 4;
    vec2 position = in_position + offsets[vertexID] * (u_map_stats.r + 0.1);

    // Scale vertex position by radius and offset by the circle position
    frag_pos = offsets[vertexID] * (u_map_stats.r + 0.1);
    frag_color = in_color;

    // Set position in screen space
    gl_Position = vec4(position, 0.0, 1.0);
}
