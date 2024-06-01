#version 330 core

uniform sampler2D Texture;
in vec2 Frag_UV;
in vec4 Frag_Color;

out vec4 ogl_FragColor;

void main()
{
    ogl_FragColor = Frag_Color * texture(Texture, Frag_UV.st);
}
