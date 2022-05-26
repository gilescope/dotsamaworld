# Dotsama Town

!\[code quality\](https://badgen.net/badge/code%20quality/yolo/green?icon=awesome)

Dotsama town is one vision of what's happening in the polkadot ecosystem.
It's an experiment to see what we can see and I look forward to evolving it
and seeing where it leads to.

PRs and suggestions (issuse) welcome along with crazy forks!

Thank you Bevy and all your plugin ecosystem for making this project a real joy to do.

! This is very experimental and under active dev. I'll clean up the code once I know what it is I'm creating. !

## What am I looking at?

There are long roads streatching into the distance - these are the parachains with the two relay chains side by side in the middle.

There are cubes which represent extrinsics (transactions) and there are spheres which represent events from those transactions.

Currently new extrinsics and events fall from the sky and land once they are finalised by grandpa.

! There may be bugs, what you see may not be correct - please double check any info you read here with external sources before doing any transactions based on this info. !

## What can I do?

As well as look around you can left click on any event or extrinsic to get some limited info on it.
If you right click on an event it will open polkadot-js at that block (and pointing to the right chain).
If you right click on an extrinsic it will open the polkadot-js decode screen for the right chain so you can see the full details.


## Prerequisites

There's probably some prerequites but if you have nix or run nixos then you can just 
`nix-shell ./shell.nix` to install whatever is needed.

## Build and serve WASM version

You can't at the moment.

I was using `trunk serve` to run up a wasm version but at the moment I'm using subxt which is not no_std
(could use smaldot or substrate-api-client instead?). Also the wasm multithreading story seems a tad early.

## Build and run native version
```
cargo run --no-default-features
```

You will need to change Cargo.toml to wayland if your using that - at the moment it's set to X11.

## Features

Note: spacemouse is on by default at the moment.

| Feature    | Description                       |
| spacemouse | n-degees of freedom mouse support |

## Donations

If you like this project please consider participating in the gitcoin grant's round 14 
where you can help projects get considerable matched funding or to https://www.mriyaaid.org/ .

## License

License: MIT/Apache2 just like rust.

## crazy idea holding ground:

https://bevyengine.org/examples/3d/spherical-area-lights/
https://github.com/therawmeatball/meme-cli

When we go back to being able to run on web would be great to be able to start from any block and show forward and backward in time and be able to jump directly there using a dedicated url.