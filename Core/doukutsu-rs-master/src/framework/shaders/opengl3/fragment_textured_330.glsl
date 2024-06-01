#version 330 core

uniform sampler2D Texture;
in vec2 Frag_UV;
in vec4 Frag_Color;

out vec4 gl_FragColor;

void main()
{
    gl_FragColor = Frag_Color * texture2D(Texture, Frag_UV.st);
}
