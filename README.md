# Dotsama Town

Dotsama town is one vision of what's happening in the polkadot ecosystem.
It's an experiment to see what we can see and I look forward to evolving it
and seeing where it leads to.

PRs and suggestions (issuse) welcome along with crazy forks!

Thank you Bevy and all your plugin ecosystem for making this project a real joy to do.

! This is very experimental and under active dev. I'll clean up the code once I know what it is I'm creating. !

## Prerequisites

There's probably some prerequites but if you have nix or run nixos then you can just 
`nix-shell ./shell.nix` to install whatever is needed.

## Build and serve WASM version

You can't at the moment.

I was using `trunk serve` to run up a wasm version but at the moment I'm using subxt which is not no_std
(could use smaldot or substrate-api-client instead?). Also the wasm multithreading story seems a tad early.

## Build and run native version
```
cargo run
```

You will need to change Cargo.toml to wayland if your using that - at the moment it's set to X11.

License: MIT/Apache2 just like rust.

## crazy idea holding ground:

https://bevyengine.org/examples/3d/spherical-area-lights/
https://github.com/therawmeatball/meme-cli