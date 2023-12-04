set -e # Exit script when any command fails
adb shell am start -n com.logisim.android/.MainActivity

APK_UID=$(adb shell pm list package -U com.logisim.android)
APK_UID_TRIMMED=${APK_UID#*uid:}
adb logcat -c
adb -d logcat -v color --uid $APK_UID_TRIMMED
