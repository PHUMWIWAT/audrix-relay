# แผนส่งเสียงจาก PC ไปมือถือด้วย Rust

ที่มา: [แชทวางแผนแอป Rust ส่งเสียง](https://chatgpt.com/share/6acb8c77-e0a0-83ec-a478-57e97ab723d7)

## โจทย์

ทำแอปแนว AudioRelay ด้วย Rust ให้เร็ว และใช้ CPU กับ RAM น้อย

- PC จับเสียงระบบ แล้วส่งไปแอปมือถือในเน็ตวงเดียวกัน
- แอปมือถือมีตัวเชื่อมต่ออยู่แล้ว ไม่ต้องสร้างแอปมือถือใหม่
- ไม่ผ่าน cloud

คำตอบที่ล็อกในแชทแล้ว:

- เครื่องส่งคือ Windows 11
- จับเสียงด้วย WASAPI loopback
- มือถือคือ Android (APK)
- โปรโตคอลที่ APK รับเสียงยังไม่รู้ ต้องตรวจก่อน
- เน้น latency ต่ำ และส่งใน LAN เท่านั้น

Rust ไม่ทำให้แอปเร็วเอง ความเร็วขึ้นกับ audio API, codec, ขนาดบัฟเฟอร์, หน่วยความจำ และวิธีส่งข้อมูล

## สถาปัตยกรรม

```text
Windows PC (เกม / เพลง / เสียงระบบ)
        |
        | WASAPI loopback capture
        v
Rust audio engine
  capture -> resample ถ้าจำเป็น -> encode -> packetize
        |
        | UDP / RTP บน Wi-Fi LAN
        v
แอปมือถือที่มีอยู่
  รับ packet -> decode -> เล่นเสียง
```

โฟกัส engine ฝั่ง PC. มือถือเป็นตัวรับที่มีอยู่แล้ว

## โปรเจกต์ที่ควรอ่าน

ศึกษาอย่างอ้างอิง ไม่ต้องยกโค้ดทั้งก้อนมาใช้

| โปรเจกต์ | ใช้ดูเรื่องอะไร | ลิงก์ |
|---|---|---|
| LAN Audio (MengBad) | ตรงโจทย์ที่สุด: Windows ไป Android, WASAPI loopback, Opus, UDP, หา PC ด้วย mDNS, เล่นต่อเมื่อปิดหน้าจอ | https://github.com/MengBad/lan-audio |
| Aqua (aquawius) | latency, packet, jitter buffer, แพ็กเก็ตหาย, อุปกรณ์เสียงเปลี่ยนระหว่างใช้ | https://github.com/aquawius/aqua |
| PulseStream | จับ WASAPI loopback แล้วส่ง TCP, หาปลายทาง, reconnect. ใช้ดูฝั่ง desktop | https://github.com/dasunNimantha/pulse-stream |
| AudioStream (0xSubhan) | PC ไป Android ด้วย Opus, RTP/UDP, adaptive jitter buffer | https://github.com/0xSubhan/AudioStream |

อ่าน LAN Audio เป็นภาพรวม และ Aqua เป็นชั้นส่งเสียง

## เทคโนโลยีฝั่ง Rust

| ส่วน | ตัวเลือก | เหตุผล |
|---|---|---|
| จับเสียง Windows | `wasapi` หรือ `cpal` | loopback เสียงระบบ. `wasapi` ตรง API ของ Windows. `cpal` เป็น abstraction หลายแพลตฟอร์ม. เลือกหลังลองกับอุปกรณ์จริง |
| บีบอัด | `audiopus` หรือ binding ของ Opus | ลดข้อมูลก่อนส่ง Wi-Fi |
| ส่งข้อมูล | `tokio` + UDP | แยกงานเน็ตออกจาก audio thread |
| บัฟเฟอร์ข้ามเธรด | `rtrb` | ring buffer ที่ไม่รบกวน real-time |
| หา PC ใน LAN | mDNS / DNS-SD | มือถือไม่ต้องกรอก IP |
| สั่งเชื่อมต่อ | WebSocket หรือ TCP | แยกคำสั่งออกจากข้อมูลเสียง |
| หน้าต่าง | Tauri 2 หรือ `egui` | GUI เรียก engine ได้. ทำทีหลัง CLI |

เอกสารก่อนลงมือ:

- [WASAPI loopback recording](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)
- [wasapi crate](https://docs.rs/wasapi/latest/wasapi/)
- [cpal](https://docs.rs/cpal/latest/cpal/)

## ส่งแบบไหนกินเครื่องน้อยกว่า

อย่าวัดจาก CPU อย่างเดียว

**แบบ A — PCM16 ไม่บีบอัด.** เริ่มง่าย. ไม่มี encode/decode จึงเพิ่ม CPU น้อย แต่กินเน็ตมากกว่า. Stereo 48 kHz 16-bit = 1.536 Mbps หรือประมาณ 192 KB/s ก่อน overhead

**แบบ B — Opus.** ใช้จริง. บีบก่อนส่ง เหมาะกับ Wi-Fi. เริ่มที่ 96–128 kbps สำหรับ stereo แล้วปรับจากคุณภาพเสียงกับ CPU ที่วัดได้. แลกด้วยงาน encode/decode

ลำดับในแผน: ส่ง PCM16 ก่อน เพื่อแยกปัญหาเน็ตออกจากปัญหา codec. เสียงถึงแล้วค่อยใส่ Opus

## ให้ engine กิน CPU และ RAM น้อย

1. แยก audio thread ออกจาก network thread. เธรดจับเสียงต้องไม่รอ UDP หรือ GUI
2. ไม่ allocate ใน audio callback. เตรียมบัฟเฟอร์ไว้ก่อน และใช้ ring buffer ขนาดจำกัด
3. ไม่ส่งทีละ sample. รวมเป็น frame ก่อนส่ง
4. บัฟเฟอร์อย่าใหญ่เกิน (หน่วง) และอย่าเล็กเกิน (เสียงขาดเมื่อ Wi-Fi jitter)
5. ไม่มีมือถือรับแล้วไม่ encode และไม่ส่ง
6. วัดจาก release build: CPU, RAM, latency, packet loss, เสียงขาด. อย่าใช้ขนาดไฟล์ exe เป็นตัววัดความเร็ว

ฝั่ง Windows เริ่มจากโปรแกรม tray เล็ก ๆ ได้ทีหลัง. ฝั่งมือถือใช้การเล่นเสียงแบบ native และบริการเบื้องหลังของระบบนั้น

เป้าหมาย latency ของรุ่นแรก: ต่ำกว่า 100 ms. แล้วค่อยลอง 30–50 ms เมื่อเน็ตและเครื่องรับไหว. เป็นเป้าทดสอบ ไม่ใช่ค่าที่การันตีทุกเครื่อง

## แผนลงมือ

1. **จับเสียง.** Rust CLI บน Windows จับเสียงระบบด้วย WASAPI. ดู sample rate, channels, buffer. ยังไม่ทำ GUI
2. **ส่งใน LAN.** ส่ง PCM16 ทาง UDP ไปตัวรับที่มีอยู่. วัดเสียงถูกไหม แพ็กเก็ตหายไหม หน่วงเท่าไร
3. **ปรับ.** ใส่ Opus, jitter buffer, เชื่อมใหม่ได้. วัด CPU/RAM ของ PCM16 เทียบ Opus
4. **แอป desktop.** เลือกอุปกรณ์เสียง, สถานะมือถือ, ระดับเสียง, latency, เปิดตอน Windows เริ่ม

## ตรวจ APK ก่อนเขียนตัวส่ง

ยังไม่รู้ว่า APK รับเสียงทางอะไร. อย่าเดาว่าเป็น UDP หรือ Opus แล้วเขียน engine ให้ตรงกับของที่เดา

บน Windows 11:

1. เก็บสำเนาไฟล์ APK
2. เปิดด้วย [JADX](https://github.com/skylot/jadx) ดูโค้ด Java/Kotlin ที่ถอดได้
3. ใช้ [Android platform-tools](https://developer.android.com/tools/releases/platform-tools) และ `adb` ตอนแอปเปิดรับเสียง
4. หา protocol, port, codec, sample rate และวิธีเริ่มเล่น จากโค้ดและจากการลองจริง

```powershell
adb version
adb devices
adb shell pm list packages -3
adb logcat
```

อย่าตัดสิน codec จากดักแพ็กเก็ตอย่างเดียว. ข้อมูลอาจถูกเข้ารหัสหรือถูกห่ออีกชั้น. ตรวจเฉพาะแอปและเครื่องที่มีสิทธิ์ตรวจ

## โครงโปรเจกต์ที่แผนแนะนำ

CLI ก่อน. อย่าผูก engine กับหน้าต่าง

```text
pc-audio-stream/
├── Cargo.toml
├── crates/
│   ├── audio-capture/      # WASAPI loopback
│   ├── audio-codec/        # Opus / PCM16
│   ├── audio-transport/    # UDP / session
│   ├── audio-protocol/     # รูปแบบแพ็กเก็ต / เวอร์ชัน
│   └── audio-core/         # pipeline / lifecycle
├── apps/
│   └── windows-sender/     # CLI ก่อน, tray ทีหลัง
├── tests/
│   ├── protocol_tests.rs
│   └── audio_pipeline_tests.rs
└── docs/
    ├── protocol.md
    └── performance.md
```

## เกณฑ์ผ่านของต้นแบบ

| รายการ | เกณฑ์เริ่ม |
|---|---|
| จับเสียง | เสียงระบบต่อเนื่อง 30 นาที |
| ความหน่วง | ต่ำกว่า 100 ms ก่อน แล้วค่อยลด |
| CPU / RAM | จดค่า idle และตอนส่ง เพื่อเทียบ |
| แพ็กเก็ตหาย | มี sequence number และรายงานทุกวินาที |
| การเชื่อม | หลุดแล้วต่อใหม่ได้ |
| เบื้องหลัง | Android เล่นต่อเมื่อปิดหน้าจอ |
| LAN only | ใช้ได้โดยไม่มีอินเทอร์เน็ต และไม่ส่งออก cloud |

ทดสอบ Wi-Fi ที่มีสัญญาณรบกวน และทดสอบมือถือหลายรุ่น ก่อนแจกของจริง

UDP ไม่การันตีว่าแพ็กเก็ตถึง. ใช้ sequence number กับ jitter buffer ที่มือถือ. จำกัดพอร์ต UDP ให้รับจากเครื่องที่อนุญาต. ถ้าต้องกันการดักฟังใน LAN ให้เข้ารหัสและยืนยันตัวตนก่อน. อย่าเปิดพอร์ตของ engine ออกอินเทอร์เน็ต

## ลำดับก่อนเขียนของจริง

1. ตรวจโปรโตคอลของ APK
2. CLI จับเสียงด้วย WASAPI loopback
3. ตัวส่งให้ตรงโปรโตคอลที่ APK รับได้
4. PCM16 ก่อน
5. Opus และการลด latency หลังเสียงถึงแล้ว

ขั้นต่อไปที่แชทค้างไว้: ได้ไฟล์ APK หรือชื่อแพ็กเกจ Android แล้วค่อยล็อกโปรโตคอลฝั่ง Rust ให้เข้ากับแอปเดิม
