package com.audrix.relay

import java.util.TreeMap

class PcmChunk(val pcm: ShortArray, val sampleRate: Int, val channels: Int)

/**
 * Holds a few packets before playback so a late Wi-Fi packet is a delay,
 * not a hole. A hole is what sounds like a tearing speaker.
 */
class PcmQueue(private val targetMs: Int = 40, private val maxMs: Int = 120) {
    private val chunks = TreeMap<Long, PcmChunk>()
    private var next: Long? = null
    private var started = false
    private var bytesPerMs = 0

    fun push(seq: Long, chunk: PcmChunk) {
        if (chunk.sampleRate > 0 && chunk.channels > 0) {
            bytesPerMs = chunk.sampleRate * chunk.channels * 2 / 1000
        }
        chunks[seq] = chunk
        if (next == null) {
            next = seq
        }
        trim()
    }

    fun pop(): PcmChunk? {
        val expected = next ?: return null
        if (!started) {
            if (bufferedBytes() < targetBytes()) {
                return null
            }
            started = true
        }
        val ready = chunks.remove(expected)
        if (ready != null) {
            next = expected + 1
            return ready
        }
        val later = chunks.ceilingKey(expected + 1) ?: return null
        val size = chunks[later]?.pcm?.size ?: return null
        next = expected + 1
        return PcmChunk(ShortArray(size), chunks[later]?.sampleRate ?: 0, chunks[later]?.channels ?: 0)
    }

    fun clear() {
        chunks.clear()
        next = null
        started = false
    }

    private fun trim() {
        val limit = maxBytes()
        while (limit > 0 && bufferedBytes() > limit) {
            val oldest = chunks.firstKey()
            chunks.remove(oldest)
            next = oldest + 1
            started = true
        }
    }

    private fun bufferedBytes(): Int = chunks.values.sumOf { it.pcm.size * 2 }

    private fun targetBytes(): Int = (bytesPerMs * targetMs).coerceAtLeast(1)

    private fun maxBytes(): Int = bytesPerMs * maxMs
}
