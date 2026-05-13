cmake_minimum_required(VERSION 3.20)
project(qrd_cpp_sdk LANGUAGES CXX)

add_library(qrd_cpp_sdk INTERFACE)
target_include_directories(qrd_cpp_sdk INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/include)
