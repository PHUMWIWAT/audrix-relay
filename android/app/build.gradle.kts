plugins {
    id("com.android.application")
}

android {
    namespace = "com.audrix.relay"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.audrix.relay"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "0.1"
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}
