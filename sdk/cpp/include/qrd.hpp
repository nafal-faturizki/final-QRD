#pragma once

#include "qrd.h"
#include <string>
#include <stdexcept>
#include <utility>
#include <vector>

namespace qrd {

struct FileReader {
    std::string path;

    explicit FileReader(std::string path_value) : path(std::move(path_value)) {}

    std::string inspect_header() const {
        throw std::runtime_error("C FFI binding not added yet");
    }
};

struct FileWriter {
    std::string path;

    explicit FileWriter(std::string path_value) : path(std::move(path_value)) {}

    void write_row(const std::vector<std::pair<std::string, std::string>>& row) {
        (void)row;
        throw std::runtime_error("C FFI binding not added yet");
    }
};

} // namespace qrd
