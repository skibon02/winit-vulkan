#version 450 core

// substituted per-instance attributes
layout (location = 0) in vec2 in_position;  // Vertex position offset for each of the 4 vertices
layout (location = 1) in float in_radius;   // Circle radius (per-instance)
layout (location = 2) in vec4 in_color;     // Circle color (per-instance)

// pass to fragment shader
layout(location = 0) out vec2 frag_pos;
layout(location = 1) out vec4 frag_color;

void main() {
    // Triangle strip vertex offsets for each of the four vertices
    vec2 offsets[4] = vec2[](
    vec2(-1.0, -1.0),
    vec2( 1.0, -1.0),
    vec2(-1.0,  1.0),
    vec2( 1.0,  1.0)
    );

    int vertexID = gl_VertexIndex % 4;
    vec2 position = in_position + offsets[vertexID] * in_radius;

    // Scale vertex position by radius and offset by the circle position
    frag_pos = offsets[vertexID] * in_radius;
    frag_color = in_color;

    // Set position in screen space
    gl_Position = vec4(position, 0.0, 1.0);
}
