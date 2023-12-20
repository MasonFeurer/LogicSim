# WARNING: This script is written for Linux and will likely not work on Windows or MacOS.
# WARNING: This script has not been tested on a clean system.

# Create Android SDK Home
Create the location for the Android SDK to stay.
```sh
mkdir "$HOME/android-sdk"
```

# Install Command Line Tools
The cmdline-tools will be installed to `cmdline-tools/latest`.
```sh
cd "$HOME/android-sdk"
curl "https://dl.google.com/android/repository/commandlinetools-linux-10406996_latest.zip" -o commandlinetools-latest.zip
unzip -q ./commandlinetools-latest.zip
rm ./commandlinetools-latest.zip

# unzip created a folder called `cmdline-tools`, we want to rename that to `latest`
mv cmdline-tools latest
# we then want to move the `latest` into a new folder, `cmdline-tools`
mkdir cmdline-tools
mv latest cmdline-tools
```

# Finish setup of Android SDK
We will use the `sdkmanager` tool in the just downloaded cmdline-tools to setup the rest of the SDK.
```sh
cd cmdline-tools/latest/bin
./sdkmanager "platform-tools"
./sdkmanager "platforms;android-31"
./sdkmanager "build-tools;30.0.3"
./sdkmanager ndk-bundle
```

https://dl.google.com/android/repository/android-ndk-r26b-linux.zip

# Set Enviornemtn Variables
Add the following to the end of `~/.bashrc`.
The rest of the tutorial will use the enviornment variables.
```sh
export ANDROID_HOME="$HOME/android-sdk"
export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk-bundle"
```

Also run this to make sure that the changes apply to your current bash session.
```sh
source ~/.bashrc
```

# Make sure Java is installed
```sh
sudo apt install openjdk-17-jdk
sudo apt install openjdk-17-jre
```

# Setup rust for android
```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-apk
```
And add these to `~/.bashrc`:
```sh
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
export PATH="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"

export CARGO_ENCODED_RUSTFLAGS=-Clink-arg=--target=aarch64-linux-android31

export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android31-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android31-clang++"
```
Then run this again:
```sh
source ~/.bashrc
```

# Get Libunwind
the `libunwind` library is needed by rust.
```sh
cd ~/Downloads
curl "http://fl.us.mirror.archlinuxarm.org/aarch64/extra/libunwind-1.6.2-2-aarch64.pkg.tar.xz" -o libunwind-1.6.2-2-aarch64.pkg.tar.xz
tar fx "libunwind-1.6.2-2-aarch64.pkg.tar.xz" usr
cp "usr/lib/libunwind.so" "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/lib64/clang/11.0.5/lib/linux/aarch64"
```
