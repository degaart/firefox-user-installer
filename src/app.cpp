#include "app.h"
#include "gui.h"
#include "installer.h"
#include <curl/curl.h>
#include <iostream>

namespace fui {

    bool App::OnInit()
    {
        /* Determine if firefox is installed */

        return false;
    }

}

namespace fui::util {

    class CurlGlobal {
        private:
            CurlGlobal();
        public:
            ~CurlGlobal();
            static CurlGlobal& get();
    };

    CurlGlobal::CurlGlobal()
    {
        std::cout << "Initializing libcurl\n";
        curl_global_init(CURL_GLOBAL_ALL);
    }

    CurlGlobal::~CurlGlobal()
    {
        std::cout << "Cleaning up libcurl\n";
        curl_global_cleanup();
    }

    CurlGlobal& CurlGlobal::get()
    {
        static CurlGlobal instance;
        return instance;
    }

}

/*wxIMPLEMENT_APP(fui::App);*/

int main(int argc, char** argv)
{
    auto installDir = fui::installer::getInstallDir();
    std::cout << "Install dir: " << installDir << "\n";

    auto isInstalled = fui::installer::isInstalled();
    std::cout << "Is installed: " << isInstalled << "\n";

    /*if(!isInstalled) {
        std::cout << "Installing...\n";
        fui::installer::install();
    }*/

    fui::util::CurlGlobal::get();

    return 0;
}


