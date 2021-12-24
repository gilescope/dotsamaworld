# Template Bevy project with WebGL enabled

## Prerequisites

```
cargo install cargo-make
```

## Build and serve WASM version

Set your local ip address in `Makefile.toml` (`localhost` will work if you want to connect to your own device)
```toml
[tasks.serve]
command = "basic-http-server"
args = ["-x", "-a", "<your-ip>:4000"]
dependencies = ["build-web", "basic-http-server"]
```

```
cargo make serve
```
then point your browser to http://<your-ip>:4000/


## Build and run native version
```
cargo make run
```
