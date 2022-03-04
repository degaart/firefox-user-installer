firefox-user-installer
===========================================================================

A debian package that installs firefox for the current user.

Details
===========================================================================

This package will install an icon in the application menu,
clicking this icon will ask:

- which version of firefox the user wants to download (Firefox latest,
  Firefox Beta, Firefox Nightly, Firefox ESR, Firefox Developer edition)
- which architecture (32-bit or 64-bit)
- which language (defaults to current language)

It will then download the requested firefox and install it for the current
user. If firefox was already installed, it will just launch it normally.
Note that the installed firefox supports auto-updating, so it can always be
kept up to date, even when testing enters freeze.

Usage
===========================================================================

Download a package from the releases page, depending on whether
you are running a 32-bit or a 64-bit debian install. Then install
the package using apt:

    sudo apt install firefox_user_installer_0.1.0_amd64.deb

Replace firefox_user_installer_0.1.0_amd64.deb with the name of
the package you downloaded.

Changing the installed firefox version can be done by launching it
with the `--reset` argument:

    firefox --reset

To uninstall, you have to uninstall the firefox_user_installer package
and remove the downloaded firefox:

    sudo apt purge firefox_user_installer
    rm -r $HOME/.local/share/firefox-user-installer


