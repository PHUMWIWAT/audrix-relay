# Commit และ push

Remote: `https://github.com/PHUMWIWAT/audrix-relay.git`  
Branch ที่ใช้: `main` (track `origin/main`)

## Commit

Commit เมื่องานที่ผู้ใช้สั่งจบแล้วหนึ่งเรื่อง และ diff มีเฉพาะเรื่องนั้น

ใส่ใน commit ได้:

- โค้ดที่ทำให้เรื่องนั้นทำงาน
- เทสที่ต้องแก้เพราะการเปลี่ยนแปลงนั้น
- เอกสารหรือกติกาที่ผู้ใช้สั่งให้เขียน

ห้าม commit:

- ความลับ: `.env`, token, key, credential, รหัสผ่าน
- ของเฉพาะเครื่อง: path ส่วนตัว, log, ไฟล์ชั่วคราว
- งานที่ยังรันไม่ได้หรือยังไม่จบ
- `.agents/` และ `skills-lock.json` ยกเว้นผู้ใช้สั่งตรง ๆ
- ไฟล์ที่ไม่เกี่ยวกับเรื่องที่แก้

ข้อความ commit เป็นหนึ่งบรรทัด และบอกสิ่งที่เปลี่ยน

## Push

Push เมื่อผู้ใช้สั่ง push หรือสั่งให้อัปขึ้น GitHub เท่านั้น

- Push ไป `origin` ที่ branch `main` เท่านั้น
- ห้าม force push
- ห้าม push ถ้าใน commit มีความลับ
- ก่อน push ให้ `git status` ไม่มีไฟล์ของเรื่องนั้นค้างอยู่ และ branch คือ `main`
