# Dotsama World

![code quality](https://badgen.net/badge/code%20quality:/yolo/green?icon=awesome)

Dotsama world is one vision of what's happening in the polkadot ecosystem right now.

PRs and suggestions (issuse) welcome along with crazy forks!

Thank you Bevy and all your plugin ecosystem for making this project a real joy to do.
(Alas I had to move off bevy due to lack of instancing support but would not have got started without it!)

! Very experimental and under active dev. !

![Image](https://github.com/gilescope/dotsamaworld-visual-history/blob/main/chapter2/Screenshot%20from%202022-05-08%2010-03-11.png?raw=true)

## What am I looking at?

Pictures are stored in a separate repo to not bloat this one: 

[Dotsama World - A Visual History](https://github.com/gilescope/dotsamaworld-visual-history)

There are long roads streatching into the distance - these are the parachains with the two relay chains side by side in the middle.

There are cubes which represent extrinsics (transactions) and there are spheres which represent events from those transactions.

Currently new extrinsics and events fall from the sky and land once they are finalised by grandpa.

The colors of the parachain blocks are the same color as the relay chain that secures them.

! There may be bugs, what you see may not be correct - please double check any info you read here with external sources before doing any transactions based on this info. !

## What can I do?

As well as look around you can left click on any event or extrinsic to get some limited info on it.
If you right click on an event it will open polkadot-js at that block (and pointing to the right chain).
If you right click on an extrinsic it will open the polkadot-js decode screen for the right chain so you can see the full details.

## Keyboard controls

 - To move about: WSAD or arrow keys, with left shift to run.
 - Hold `.` to rise and `,` to lower (or space and right shift). 
 - Escape switches the mouse from being able to select something to being able to look around.
 - Tab lowers or raises the anchor so that you do or don't follow the chain.
 - roll x, z
 - yaw q, e
 - pitch [, ]
 - p = toggle to pause/unpause data feeds.
 - o = hold down for slow motion mode.

## Prerequisites

There's probably some prerequites but if you have nix or run nixos then you can just 
`nix-shell` to install whatever is needed. I've seen it running on Linux and OSX. Not yet on windows. sudo apt-get install libxcb-shape0-dev libxcb-xfixes0-dev

## Build and serve WASM version

trunk serve --release

## Build and run native version
```
cargo run --release
```

You will need to change Cargo.toml to wayland if your using that - at the moment it's set to X11.

## Features

Note: spacemouse support is not on by default.

| Feature      | Description                                        |
| ------------ | -------------------------------------------------- |
| spacemouse   | n-degrees of freedom mouse support                 |

## Recording and Playback

(Currently disabled)

It should record your journey in a file called `record.csv`. If you want to play it back, then rename it to `play.csv` and it will execute it. The format is:
| time in seconds elapsed | x | y | z | rx | ry | rz | rw |

The `r` ones are rotation as a quaternion.

## Donations

If you like this project please consider donating to https://www.mriyaaid.org/ .

## License

The cool emojis currently come from https://github.com/microsoft/fluentui-emoji and are licensed under the MIT license. (Thank you for making and open sourcing these. I've rendered them to png and baked them into one image.)

License: MIT or Apache2 at your option just like rust.