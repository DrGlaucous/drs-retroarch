#version 330 core

in vec2 Frag_UV;
in vec4 Frag_Color;

out vec4 gl_FragColor;

void main()
{
    gl_FragColor = Frag_Color;
}