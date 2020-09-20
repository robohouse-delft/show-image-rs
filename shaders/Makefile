.PHONY: all

all: shader.vert.spv uint8.frag.spv unorm8.frag.spv

%.spv: %
	glslangValidator -V -o "$@" "$<"
