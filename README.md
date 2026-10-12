<p align="center">
  <img src="assets/logo-mark.png" alt="Audrix Relay" width="160">
</p>

# Audrix Relay

Audrix Relay plays what you hear on a Windows PC through an Android phone on the same Wi-Fi. The PC captures system audio. The phone chooses which PC to listen to.

Audio stays on the local network as 16-bit PCM. Nothing is uploaded.

Current release: [v0.10.0](https://github.com/PHUMWIWAT/audrix-relay/releases/tag/v0.10.0).

## Use it

1. Put the PC and the phone on the same Wi-Fi. The network must allow devices to talk to each other.
2. On Windows, run `AudrixRelay.exe`. The window has three pages: Status, Settings, and About.
3. On the phone, open Audrix Relay and tap the PC in the list. Both screens show the delay in milliseconds.
4. Leave the phone app. Playback continues from the notification. Tap **Stop** to disconnect. The phone keeps looking for PCs.

Closing the PC window hides it in the tray. Right-click the tray icon for Show, Hide, and Exit. Exit is what stops capture. Only one PC window can listen at a time.

If the phone list stays empty, allow `AudrixRelay.exe` through Windows Firewall.

### PC window

- **Status** shows this PC's address, the connected phone, and the delay.
- **Settings** has the volume slider (0–200%) and the language switch (EN / TH). It also chooses 5 ms or 10 ms audio chunks. 10 ms is the default.
- **About** has **Install**, which copies the program to `%LOCALAPPDATA%\AudrixRelay` and adds a Start menu shortcut. **Uninstall** removes that shortcut.

### Phone

The app holds Wi-Fi awake while audio is playing, so the radio does not sleep and drop the session. A short gap does not end the connection. If the receive path dies, the phone retries, starting at 1 second and backing off to 30 seconds.

Playback waits about 80 ms so a late packet is a delay, not a hole. A missing packet is replaced with a brief silence. The level into the phone speaker is reduced so the amplifier is not driven into clipping.

## Build the PC app

Windows 11 and Rust 1.85 or newer.

```powershell
cargo build --release
```

The release program has no console. The window is the interface. The binary is `target\release\audrix-relay.exe`.

`cargo run -- capture` prints the capture format and exits. `cargo run -- send` and `cargo run -- recv` move 50 PCM packets across localhost. Those commands are for checks.

This repository pins `wasapi` 0.12 so the program links with the GNU Rust toolchain. That toolchain has no `dlltool`, which newer `wasapi` releases need.

## Build the Android app

Android Studio with SDK platform 37. The app needs Android 8 or newer.

```powershell
cd android
gradlew.bat assembleDebug
```

A release APK needs `android/keystore.properties` and a `.jks` next to it. Those files stay on the signing machine. Do not commit them. Install the APK from the file. A USB cable is not required.

## On the network

UDP port `40123`. Multi-byte numbers are little-endian.

The phone broadcasts `FIND`. A PC answers `HERE` plus its computer name. After you tap a PC, the phone sends `PING` and an `AXDR` hello. The PC answers `PONG` and streams audio to that phone. Control packets and audio packets use separate sockets, so audio does not crowd out the keepalive.

An audio packet is:

```text
u32 sequence
u32 sample_rate
u16 channels
i16 samples[]
```

The PC sends steady chunks, split into equal packets that stay under 1200 bytes so Wi-Fi does not have to fragment them. Samples pass through as captured. Only a peak louder than 16-bit can store is limited.

## Not in this version

Opus compression is not included. The work list is in `dosc/phases.md`.
