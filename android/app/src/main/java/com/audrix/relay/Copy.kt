package com.audrix.relay

object Copy {
    var thai: Boolean = true

    fun empty() = if (thai) {
        "ยังไม่เห็นเครื่องในวงนี้ เปิดโปรแกรมบน PC ก่อน"
    } else {
        "No PC on this network. Open the PC app first."
    }

    fun tap() = if (thai) "แตะชื่อเครื่องเพื่อฟังเสียง" else "Tap a PC to listen"

    fun playing(ip: String) = if (thai) "กำลังเล่นจาก $ip" else "Playing from $ip"

    fun stop() = if (thai) "หยุด" else "Stop"

    fun searching() = if (thai) "กำลังหาเครื่อง" else "Looking for a PC"

    fun connecting() = if (thai) "กำลังเชื่อม" else "Connecting"

    fun playingShort() = if (thai) "กำลังเล่น" else "Playing"
}
