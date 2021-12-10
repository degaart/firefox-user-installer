#include "util.h"
#include <stdlib.h>
#include <stdexcept>

namespace fui::util {

    std::filesystem::path getHomeDir()
    {
        char* homeDir = getenv("HOME");
        if(!homeDir) {
            throw std::runtime_error("$HOME is undefined");
        }
        
        return std::filesystem::path(homeDir);
    }

}
