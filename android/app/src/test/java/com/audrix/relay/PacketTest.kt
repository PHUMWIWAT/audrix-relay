package com.audrix.relay

import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PacketTest {
    @Test
    fun lossCountCountsAGapAndIgnoresADuplicate() {
        val count = LossCount()
        count.observe(0u)
        count.observe(2u)
        count.observe(1u)
        assertEquals(2, count.received)
        assertEquals(1, count.lost)
        assertEquals(1, count.duplicates)
    }

    @Test
    fun parseReadsLittleEndianHeaderAndSamples() {
        val bytes = ByteBuffer.allocate(18).order(ByteOrder.LITTLE_ENDIAN)
        bytes.putInt(7)
        bytes.putInt(48000)
        bytes.putShort(2)
        bytes.putShort(1)
        bytes.putShort(-2)
        bytes.putShort(3)
        bytes.putShort(4)
        val parsed = parsePacket(bytes.array(), bytes.array().size)
        requireNotNull(parsed)
        assertEquals(7u, parsed.seq)
        assertEquals(48000, parsed.sampleRate)
        assertEquals(2, parsed.channels)
        assertEquals(1, parsed.pcm[0].toInt())
        assertEquals(-2, parsed.pcm[1].toInt())
    }

    @Test
    fun parseRejectsAShortHeader() {
        assertNull(parsePacket(ByteArray(6), 6))
    }
}
