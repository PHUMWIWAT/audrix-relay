# Audrix Relay

Audrix Relay sends what you hear on a Windows PC to an Android phone on the same Wi-Fi. The PC captures system audio. The phone chooses which PC to play.

Nothing is uploaded. Audio stays on the local network as uncompressed PCM.

## Use it

1. Put the PC and the phone on the same Wi-Fi.
2. On the PC, run `AudrixRelay.exe` or `target\release\audrix-relay.exe`. The window shows the logo and this machine's IP. Closing the window hides it. Right-click the tray icon for Show, Hide, and Exit. Exit is what stops capture.
3. On the phone, open Audrix Relay. Tap the PC in the list. The delay is shown in milliseconds on both screens.
4. Leave the phone app. Playback continues. Tap **หยุด** in the app, or **หยุด** on the notification, to disconnect. The phone keeps looking for PCs.

If the phone list stays empty, allow `audrix-relay.exe` through Windows Firewall. Only one PC window can listen at a time. Close the old window before opening a new one.

## Build the PC app

Windows 11, Rust 1.85 or newer.

```powershell
cargo build --release
```

The release binary has no console. The window is the interface.

`cargo run -- capture` prints the capture format and exits. `cargo run -- send` and `cargo run -- recv` move 50 PCM packets across localhost. Those commands are for checks. Normal use is the window.

This repository pins `wasapi` 0.12 so the program links with the GNU Rust toolchain. That toolchain has no `dlltool`, which newer `wasapi` releases need.

## Build the Android app

Android Studio, with SDK platform 37. The app needs Android 8 or newer.

Open the `android` folder. A debug build needs no signing file:

```powershell
cd android
gradlew.bat assembleDebug
```

A release APK needs `android/keystore.properties` and a `.jks` file next to it. Those files stay on the machine that signs the app. Do not commit them. Install the release APK from the file. The phone does not need a USB cable.

## Packet

UDP port `40123`. Multi-byte fields are little-endian.

Audio:

```text
u32 sequence
u32 sample_rate
u16 channels
i16 samples[]
```

The phone finds PCs by broadcasting `FIND`. A PC answers `HERE` plus its computer name. After you tap a PC, the phone sends `PING` and a short `AXDR` hello. The PC answers `PONG` and then streams audio to that phone. The hello carries the measured delay so the PC window can show the same number.

## What is not in this version

Opus, a jitter buffer, and a tray icon are still planned. See `dosc/phases.md`.
