cmake_minimum_required(VERSION 3.16)

project(run_generate)

set(CMAKE_CXX_STANDARD 17)

add_executable(run_generate main_generate.cpp)

target_include_directories(run_generate PRIVATE
    ../llama.cpp/include
    ../llama.cpp/ggml/include
)

target_link_directories(run_generate PRIVATE
    ../llama.cpp/build/bin
)

target_link_libraries(run_generate PRIVATE
    llama
    ggml
    ggml-base
)

