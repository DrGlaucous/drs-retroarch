#version 330 core

in vec2 Frag_UV;
in vec4 Frag_Color;

out vec4 ogl_FragColor;

void main()
{
    ogl_FragColor = Frag_Color;
}
