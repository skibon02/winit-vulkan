#version 450 core

// Uniform decls
layout (binding=2) uniform sampler2D tex;

// Inputs from the vertex shader
layout (location=0) in vec2 frag_pos;
layout (location=1) in vec4 frag_color;


// Target output color
layout(location = 0) out vec4 outColor;

void main() {
    float dist_sq = dot(frag_pos, frag_pos);
    float alpha = smoothstep(1.0, 0.0, dist_sq);
    outColor = vec4(frag_color.rgb, frag_color.a * alpha) * texture(tex, frag_pos);

    if (dist_sq > 1.0) {
        discard;
    }
}
