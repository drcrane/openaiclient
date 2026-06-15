cmake_minimum_required(VERSION 3.16)

project(main_generate)

set(CMAKE_CXX_STANDARD 17)

add_executable(main_generate main_generate.cpp)

target_include_directories(main_generate PRIVATE
    ../llama.cpp/include
    ../llama.cpp/ggml/include
)

target_link_directories(main_generate PRIVATE
    ../llama.cpp/build/bin
)

target_link_libraries(main_generate PRIVATE
    llama
    ggml
    ggml-base
)

