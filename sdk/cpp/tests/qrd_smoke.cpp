#include "qrd.hpp"

int main() {
    qrd::FileReader reader("example.qrd");
    qrd::FileWriter writer("output.qrd");

    (void)reader;
    (void)writer;
    return 0;
}
