#!/bin/bash

if ! command -v glslc &> /dev/null
then
    echo "glslc could not be found"
    exit
fi

glslc shader.vert -o shader.vert.spv -O --target-env=vulkan1.2
glslc shader.frag -o shader.frag.spv -O --target-env=vulkan1.2

glslc egui.vert -o egui.vert.spv -O --target-env=vulkan1.2
glslc egui.frag -o egui.frag.spv -O --target-env=vulkan1.2
