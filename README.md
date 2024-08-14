## LogicSim
A hardware-accelerated Logic Gate Simulator.

## How to Run
There are not currently any pre-built binaries/APKS for the app, so for any platform, it must be built from source.

## Compiling From Source
To compile the code for any platform you will need `cargo` from the [Rust toolchain](https://www.rust-lang.org/).
Below are the platforms that the app is currently implemented(or planned) for.

#### Desktop
To compile and run the app on Windows, Linux, or MacOS:
```sh
git clone "https://github.com/MasonFeurer/LogicSim.git"
cd LogicSim
cargo r -rp mlsim-desktop
```

On Linux, you may have to install a few packages first:
```sh
sudo apt install libglib2.0-dev
sudo apt install libatk1.0-dev
sudo apt install libcairo2-dev
sudo apt install libpango1.0-dev
sudo apt install librust-gdk-dev
```

#### Web
Web has been temporarily removed for a codebase rewrite.

#### Android
LogicSim for Android devices can be easily compiled and ran on a connected Android device with my tool [JanoCLI](https://github.com/MasonFeurer/Jano?tab=readme-ov-file#jano-cli).

```sh
git clone "https://github.com/MasonFeurer/LogicSim.git"
cd LogicSim
jano run -p mlsim-android
```

#### IOS
There are plans to integrate the app for IOS, but there has currently been no progress towards this.

## Creating New Integrations
The app is structured in a way such that it can be integerated into any application that can render graphics with `wgpu`.
The UI rendering and circuit simulation is all handled in `mlsim-common`, and application lifetime is handled by the integration.
You can look at `mlsim-desktop` or `mlsim-android` for an example on how to integrate the common library.
