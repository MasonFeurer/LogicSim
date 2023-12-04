set -e # Exit script when any command fails

cargo ndk -t arm64-v8a -o app/src/main/jniLibs/  build
cargo ndk -t armeabi-v7a -o app/src/main/jniLibs/  build
./gradlew build
./gradlew installDebug
