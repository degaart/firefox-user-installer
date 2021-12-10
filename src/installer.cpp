#include "installer.h"
#include "util.h"

namespace fui::installer {

    std::filesystem::path getInstallDir()
    {
        auto result = util::getHomeDir();
        result.append(".local/firefox");
        return result;
    }

    bool isInstalled()
    {
        auto installDir = getInstallDir();

        std::error_code error;
        return std::filesystem::is_directory(installDir, error);
    }

    void install()
    {

    }

}