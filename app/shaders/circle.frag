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
    vec4 tex_color = texture(tex, frag_pos * 0.5 + 0.5);
    if (tex_color[0] > 0.7 && tex_color[1] > 0.7 && tex_color[2] > 0.7) {
        discard;
    }
    outColor = vec4(tex_color.rgb, alpha);

    if (dist_sq > 1.0) {
        discard;
    }
}
