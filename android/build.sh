set -e # Exit script when any command fails

cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  build # aarch64-linux-android
cargo ndk -t armeabi-v7a -o app/src/main/jniLibs/  build # armv7-linux-androideabi
./gradlew build
./gradlew installDebug
