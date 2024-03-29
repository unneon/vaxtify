# Vaxtify [![](https://img.shields.io/github/workflow/status/unneon/vaxtify/Continuous%20Integration?logo=github-actions&logoColor=white)](https://github.com/unneon/vaxtify/actions) [![](https://img.shields.io/github/license/unneon/vaxtify?color=success&logo=github)](https://github.com/unneon/vaxtify)

Vaxtify is a distraction blocker for the stubborn.
Basically, it automatically closes websites that you may spend too much time on.
To spend your time more productively while still being able to relax sometimes, these rules can be scheduled to only apply during certain hours.
In case a fixed schedule is not enough, it can be configured to let you manually unlock access for a set time, at most once a day or once every few hours.

## Getting started

Vaxtify works on Linux, and the required extension is packaged for Firefox.

### Install the program

If you use Arch Linux, you can install Vaxtify from AUR using `yay -S vaxtify`.
Otherwise, install Rust, lidbus-1-dev or equivalent, manually run the build command and copy files as shown in the [PKGBUILD](misc/arch-packaging/PKGBUILD) script, and run `systemctl --user daemon-reload` to load the installed service files.

### Configure

In Vaxtify, you first define categories which group sites you may want to block.
These can include domain names, subreddits, github repositories, and custom regexes which will be matched on the page URL.

To block sites from these categories, you define rules.
Each rule has a list of categories it applies to, and it can optionally have a set period during which it should not apply.

If you want more fine-grained control over what you can access, you can use permits.
Each permit has a name, and a list of categories it will allow you to visit (despite them being blocked by rules).
Optionally, you can set how long they will last by default, how long they can last at most, how rarely they can be used, and a set period during which they can be used.

```kdl
// General settings, such as specifying whether an empty tab should be created
// after closing the last one.
// prevent-browser-close

// Define a new category, called "memes". Pay attention whether URLs include www
// or not. Subreddits are case insensitive. Each line can be omitted if empty.
category "memes" {
    domains "www.youtube.com"
    subreddits "all" "funny"
    githubs "unneon/icie"
    regexes r"\w+\+memes"
}

// Create a rule that applies to everything from "meme" category. It will be
// active before 23:30 and after 0:00 in local time. If you want a rule to be
// always active, remove the "allowed" block.
rule "toomanymemes" {
    allowed {
        since hour=23 min=30
        until hour=0
    }
    categories "memes"
}

// Define a new permit, called "dailymemes". It will stop blocking the category
// for 15 minutes when used, and can only be used between 20:00 and 0:00.
permit "dailymemes" {
    length mins=15
    cooldown hours=20
    available {
        since hour=20
        until hour=0
    }
    categories "memes"
}
```

Copy this file to ~/.config/vaxtify.kdl.
I suggest to check if everything works before editing it.

### Install the browser extension

If you installed Vaxtify from AUR, restart Firefox and activate the extension in the application menu.
Otherwise, view the [latest release](https://github.com/unneon/vaxtify/releases/latest) on GitHub and click on the vaxtify.xpi asset to install it.

### Enjoy

The daemon will launch automatically, as soon as you install the web extension (thanks to D-Bus activation).
Assuming you have not modified the default config yet, you can check that going to [youtube](https://youtube.com), [r/funny](https://www.reddit.com/r/funny), [github.com/unneon/icie](https://github.com/unneon/icie) or Googling "cat memes" will immediately close the tab, unless it's between 23:30 and 0:00 in local time.

To use permits, run `vaxtify permit dailymemes` and check that the websites won't be blocked for 15 minutes.
You can also run `vaxtify permit dailymemes end` to end it quicker than planned.

After changing the configuration, run `systemctl --user reload vaxtify` to reload the configuration file without resetting cooldowns.
