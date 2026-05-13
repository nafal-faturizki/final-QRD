#include "qrd.hpp"
#include <iostream>

/**
 * QRD C++ SDK implementation
 *
 * This file provides any C++ specific implementations needed beyond
 * the header-only definitions in qrd.hpp. Currently most functionality
 * is header-only for optimal inlining and template specialization.
 *
 * Initialization and library setup can be added here if needed.
 */

namespace qrd {

/**
 * Initialize the QRD C++ library
 * Should be called once at application startup
 */
void initialize() {
    // Placeholder for any global initialization needed
    // In production, might:
    // - Load the Rust FFI library
    // - Initialize cryptographic providers
    // - Set up thread-local storage
}

/**
 * Get library version information
 */
const char* version() {
    return "0.1.0";
}

}  // namespace qrd
