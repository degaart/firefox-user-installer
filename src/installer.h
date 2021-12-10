#pragma once

#include <filesystem>

namespace fui::installer {
    
    std::filesystem::path getInstallDir();
    bool isInstalled();
    void install();
    
}

