# Vaxtify [![](https://img.shields.io/github/workflow/status/pustaczek/vaxtify/Continuous%20Integration?logo=github-actions&logoColor=white)](https://github.com/pustaczek/vaxtify/actions) [![](https://img.shields.io/codecov/c/github/pustaczek/vaxtify?logo=codecov&logoColor=white)](https://codecov.io/gh/pustaczek/vaxtify) [![](https://img.shields.io/github/license/pustaczek/vaxtify?color=success&logo=github)](https://github.com/pustaczek/vaxtify)

Vaxtify is a distraction blocker for the stubborn.
Basically, it automatically closes websites that you may spend too much time on.
To spend your time more productively while still being able to relax sometimes, these rules can be scheduled to only apply during certain hours.
In case a fixed schedule is not enough, it can be configured to let you manually unlock access for a set time, at most once a day or once every few hours.

## Getting started

Vaxtify works on Linux, and the required extension is packaged for Firefox.

### Install the program

If you use Arch Linux, you can use a prepared [PKGBUILD](misc/arch-packaging/PKGBUILD) script.
Otherwise, install Rust, lidbus-1-dev or equivalent, and manually run the build command and copy files as shown in the linked script.

Vaxtify daemon will be managed by systemd, so run `systemctl --user daemon-reload` to load the installed service files.

### Configure

In Vaxtify, you first define categories which group sites you may want to block.
These can include domain names, subreddits, github repositories, and custom regexes which will be matched on the page URL.

To block sites from these categories, you define rules.
Each rule has a list of categories it applies to, and it can optionally have a set period during which it should not apply.

If you want more fine-grained control over what you can access, you can use permits.
Each permit has a name, and a list of categories it will allow you to visit (despite them being blocked by rules).
Optionally, you can set how long they will last by default, how long they can last at most, and how rarely they can be used.

```toml
[general]
# If the tab being blocked is the last one in the browser process, create a new empty one.
prevent_browser_close = true

# Define a new category, called "memes".
[category.memes]
  # Include any youtube videos at all.
  # Pay attention whether URL in the browser includes www. or not.
  domains = ["www.youtube.com"]
  # Include two specific subreddits. Case insensitive.
  subreddits = ["all", "funny"]
  # Include some github repository.
  # Each line can be omitted, if e.g. you have no repos to block.
  githubs = ["pustaczek/icie"]
  # Include anything containing "something+memes", like Google searches.
  # See https://regexr.com/ to learn how to write regexes.
  regexes = ["\\w+\\+memes"]

# Create a rule that applies to everything from "meme" category.
# It will be active before 22:00 and after 23:30.
[rule.toomanymemes]
  # To make an always active rule, remove both of these lines.
  allowed.since = { hour = 22, min = 0 }
  allowed.until = { hour = 23, min = 30 }
  # Rule will apply only to categories you put in this field.
  categories = ["memes"]

# Define a new permit, called "dailymemes".
[permit.dailymemes]
  # It will be active for 15 minutes at most.
  length.default = { mins = 15 }
  length.maximum = { mins = 15 }
  # After using it, you will need to wait a day before using it again.
  cooldown = { hours = 20 }
  # Using it will let you look at memes regardless of the time.
  categories = ["memes"]
```

Copy this file to ~/.config/vaxtify.toml.
I suggest to check if everything works before editing it.

### Install the browser extension

View the [latest release](https://github.com/pustaczek/vaxtify/releases/latest) on github and click on the vaxtify.xpi asset.
Your browser should download and install it after asking you for permission.

### Enjoy

The daemon will launch automatically, as soon as you install the web extension (thanks to D-Bus activation).
Assuming you have not modified the default config yet, you can check that going to [youtube](https://youtube.com), [r/funny](https://www.reddit.com/r/funny), [github.com/pustaczek/icie](https://github.com/pustaczek/icie) or Googling "cat memes" will immediately close the tab, unless it's between 22:00 and 23:30 in local time.

To use permits, run `vaxtify permit dailymemes` and check that the websites won't be blocked for 15 minutes.
You can also run `vaxtify permit dailymemes 2min` to only activate it for 2 minutes, or `vaxtify permit dailymemes end` to end it quicker than planned.

After changing the configuration, run `systemctl --user restart vaxtify` to restart Vaxtify.
Be aware this will also reset all cooldowns, the daemon does not store state between runs yet.
