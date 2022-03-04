# firefox-user-installer

A debian package to automatically install the latest firefox on debian. With autoupdate support.

## Details

This package will install an icon in the application menu, clicking this
icon will download the firefox from mozilla's site and install it for the current
user. If firefox was already installed, it will just launch it normally.
Note that the installed firefox supports auto-updating, so it can always be
kept up to date, even when testing enters freeze.

## Screenshots

![](doc/screen01.png)
![](doc/screen02.png)
![](doc/screen03.png)
![](doc/screen04.png)
![](doc/screen05.png)

## Requirements

- An x86 or amd64 processor
- debian 10 and above (submit an issue if debian 9 support is needed)

## Usage

Download a package from this project's [releases page](https://github.com/degaart/firefox-user-installer/releases) (https://github.com/degaart/firefox-user-installer/releases), depending on whether you are
running a 32-bit or a 64-bit debian install.

Then install the package using apt:

    sudo apt install ./firefox_user_installer_0.1.0_amd64.deb


Replace `firefox_user_installer_0.1.0_amd64.deb` with the name of the package
you downloaded.

Changing the installed firefox version can be done by launching it
with the `--reset` argument:

    firefox --reset

To uninstall, you have to uninstall the firefox_user_installer package and
remove the downloaded firefox:

    sudo apt purge firefox_user_installer
    rm -r $HOME/.local/share/firefox-user-installer


