#include "qrd.hpp"
#include <cassert>
#include <fstream>
#include <iostream>
#include <sstream>
#include <cstring>

// Utility to create temporary files
std::string create_temp_qrd() {
    static int counter = 0;
    std::ostringstream oss;
    oss << "/tmp/test_qrd_" << counter++ << ".qrd";
    return oss.str();
}

void test_file_reader_validation() {
    std::cout << "Testing FileReader validation..." << std::endl;

    // Test non-existent file
    try {
        qrd::FileReader reader("/nonexistent/path/file.qrd");
        assert(false && "Should throw on non-existent file");
    } catch (const std::runtime_error& e) {
        assert(std::string(e.what()).find("Cannot open") != std::string::npos);
    }

    // Test file too short
    std::string temp = create_temp_qrd();
    {
        std::ofstream file(temp, std::ios::binary);
        file.write("SHORT", 5);
    }
    try {
        qrd::FileReader reader(temp);
        assert(false && "Should throw on short file");
    } catch (const std::runtime_error& e) {
        assert(std::string(e.what()).find("too short") != std::string::npos);
    }
    remove(temp.c_str());
}

void test_inspect_header() {
    std::cout << "Testing header inspection..." << std::endl;

    std::string temp = create_temp_qrd();
    {
        std::ofstream file(temp, std::ios::binary);
        uint8_t header[32] = {0};
        header[0] = 0x51;  // 'Q'
        header[1] = 0x52;  // 'R'
        header[2] = 0x44;  // 'D'
        header[4] = 1;     // format_major = 1
        header[6] = 0;     // format_minor = 0
        header[8] = 0;
        header[9] = 1;
        header[10] = 2;
        header[11] = 3;
        header[12] = 4;
        header[13] = 5;
        header[14] = 6;
        header[15] = 7;
        file.write(reinterpret_cast<const char*>(header), 32);
    }

    try {
        qrd::FileReader reader(temp);
        auto header = reader.inspect_header();
        assert(header.format_major == 1);
        assert(header.format_minor == 0);
        assert(header.schema_id.size() == 8);
        std::cout << "  Header inspection passed" << std::endl;
    } catch (const std::exception& e) {
        std::cerr << "Exception: " << e.what() << std::endl;
        assert(false);
    }

    remove(temp.c_str());
}

void test_inspect_header_bad_magic() {
    std::cout << "Testing header validation..." << std::endl;

    std::string temp = create_temp_qrd();
    {
        std::ofstream file(temp, std::ios::binary);
        uint8_t header[32] = {0};
        header[0] = 0x58;  // 'X' (not 'Q')
        file.write(reinterpret_cast<const char*>(header), 32);
    }

    try {
        qrd::FileReader reader(temp);
        reader.inspect_header();
        assert(false && "Should throw on bad magic");
    } catch (const std::runtime_error& e) {
        assert(std::string(e.what()).find("magic") != std::string::npos);
    }

    remove(temp.c_str());
}

void test_file_writer_validation() {
    std::cout << "Testing FileWriter validation..." << std::endl;

    // Empty schema should throw
    try {
        qrd::FileWriter::Schema empty;
        qrd::FileWriter writer(create_temp_qrd(), empty);
        assert(false && "Should throw on empty schema");
    } catch (const std::runtime_error& e) {
        assert(std::string(e.what()).find("empty") != std::string::npos);
    }

    // Valid schema should work
    qrd::FileWriter::Schema schema{
        {"id", "int32"},
        {"value", "float32"}
    };
    std::string temp = create_temp_qrd();
    qrd::FileWriter writer(temp, schema);
    assert(writer.path() == temp);
    std::cout << "  Schema validation passed" << std::endl;
    remove(temp.c_str());
}

void test_file_writer_row_schema() {
    std::cout << "Testing FileWriter row schema..." << std::endl;

    qrd::FileWriter::Schema schema{
        {"id", "int32"},
        {"name", "utf8"}
    };
    std::string temp = create_temp_qrd();
    qrd::FileWriter writer(temp, schema);

    // Valid row
    qrd::FileWriter::Row valid_row{
        {"id", "1"},
        {"name", "test"}
    };
    writer.write_row(valid_row);

    // Invalid row (missing field)
    try {
        qrd::FileWriter::Row invalid_row{
            {"id", "2"}
        };
        writer.write_row(invalid_row);
        assert(false && "Should throw on missing field");
    } catch (const std::invalid_argument& e) {
        assert(std::string(e.what()).find("Missing") != std::string::npos);
    }

    remove(temp.c_str());
}

void test_file_writer_finish() {
    std::cout << "Testing FileWriter finish..." << std::endl;

    qrd::FileWriter::Schema schema{
        {"id", "int32"}
    };
    std::string temp = create_temp_qrd();
    qrd::FileWriter writer(temp, schema);

    // First finish
    writer.finish();

    // Second finish should throw
    try {
        writer.finish();
        assert(false && "Should throw on duplicate finish");
    } catch (const std::runtime_error& e) {
        assert(std::string(e.what()).find("already finished") != std::string::npos);
    }

    // Write after finish should throw
    try {
        qrd::FileWriter::Row row{{"id", "1"}};
        writer.write_row(row);
        assert(false && "Should throw on write after finish");
    } catch (const std::runtime_error& e) {
        assert(std::string(e.what()).find("already finished") != std::string::npos);
    }

    remove(temp.c_str());
}

void test_convenience_functions() {
    std::cout << "Testing convenience functions..." << std::endl;

    std::string temp = create_temp_qrd();
    {
        std::ofstream file(temp, std::ios::binary);
        uint8_t header[32] = {0};
        header[0] = 0x51;  // 'Q'
        header[1] = 0x52;  // 'R'
        header[2] = 0x44;  // 'D'
        file.write(reinterpret_cast<const char*>(header), 32);
    }

    auto header = qrd::inspect_header(temp);
    assert(header.format_major == 0);

    remove(temp.c_str());
}

int main() {
    try {
        test_file_reader_validation();
        test_inspect_header();
        test_inspect_header_bad_magic();
        test_file_writer_validation();
        test_file_writer_row_schema();
        test_file_writer_finish();
        test_convenience_functions();

        std::cout << "\nAll tests passed!" << std::endl;
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "Test failed: " << e.what() << std::endl;
        return 1;
    }
}
