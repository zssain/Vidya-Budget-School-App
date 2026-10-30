# KeystorePlugin is instantiated over JNI and its @Command methods are found by
# reflection. tauri-android's consumer rules already keep @TauriPlugin classes;
# this is a belt-and-braces keep for R8 in release builds.
-keep class app.vidya.plugin.KeystorePlugin { *; }
