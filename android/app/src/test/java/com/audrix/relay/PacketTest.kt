package com.audrix.relay

import java.io.File
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
        val bytes = sample("valid.bin")
        val parsed = parsePacket(bytes, bytes.size)
        requireNotNull(parsed)
        assertEquals(7u, parsed.seq)
        assertEquals(48000, parsed.sampleRate)
        assertEquals(2, parsed.channels)
        assertEquals(1, parsed.pcm[0].toInt())
        assertEquals(-2, parsed.pcm[1].toInt())
    }

    @Test
    fun parseRejectsAShortHeader() {
        val bytes = sample("short.bin")
        assertNull(parsePacket(bytes, bytes.size))
    }

    private fun sample(name: String): ByteArray {
        var dir = File(System.getProperty("user.dir"))
        repeat(5) {
            val candidate = File(dir, "dosc/packet-samples/$name")
            if (candidate.exists()) {
                return candidate.readBytes()
            }
            dir = dir.parentFile ?: return@repeat
        }
        error("missing dosc/packet-samples/$name from ${System.getProperty("user.dir")}")
    }
}
