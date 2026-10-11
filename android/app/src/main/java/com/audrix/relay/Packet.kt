package com.audrix.relay

import java.nio.ByteBuffer
import java.nio.ByteOrder

class PcmPacket(
    val seq: UInt,
    val sampleRate: Int,
    val channels: Int,
    val pcm: ShortArray,
)

fun parsePacket(data: ByteArray, length: Int): PcmPacket? {
    if (length < 10 || (length - 10) % 2 != 0) {
        return null
    }
    val buf = ByteBuffer.wrap(data, 0, length).order(ByteOrder.LITTLE_ENDIAN)
    val seq = buf.int.toUInt()
    val sampleRate = buf.int
    val channels = buf.short.toInt() and 0xffff
    if (sampleRate <= 0 || channels == 0) {
        return null
    }
    val count = (length - 10) / 2
    if (count % channels != 0) {
        return null
    }
    val pcm = ShortArray(count)
    for (i in 0 until count) {
        pcm[i] = buf.short
    }
    return PcmPacket(seq, sampleRate, channels, pcm)
}

class LossCount {
    private var next: UInt? = null
    var received: Int = 0
        private set
    var lost: Int = 0
        private set
    var duplicates: Int = 0
        private set

    fun observe(seq: UInt) {
        val expected = next
        when {
            expected == null -> {
                received += 1
                next = seq + 1u
            }
            seq == expected -> {
                received += 1
                next = seq + 1u
            }
            seq > expected -> {
                received += 1
                lost += (seq - expected).toInt()
                next = seq + 1u
            }
            else -> duplicates += 1
        }
    }
}
