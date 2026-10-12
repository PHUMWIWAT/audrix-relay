package com.audrix.relay

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PcmQueueTest {
    @Test
    fun waitsUntilFortyMillisecondsAreBuffered() {
        val queue = PcmQueue()
        queue.push(0, chunk(100))
        assertNull(queue.pop())
        val first = queue.popAfter(chunk(3840))
        assertEquals(100, first.pcm.size)
    }

    @Test
    fun aMissingPacketBecomesSilenceInsteadOfAJump() {
        val queue = PcmQueue()
        queue.push(0, chunk(3840))
        queue.pop()
        queue.push(2, chunk(3840))
        val concealed = queue.pop()
        requireNotNull(concealed)
        assertEquals(0, concealed.pcm[0].toInt())
        val third = queue.pop()
        requireNotNull(third)
        assertEquals(3, third.pcm[0].toInt())
    }

    private fun PcmQueue.popAfter(full: PcmChunk): PcmChunk {
        push(1, full)
        val played = pop()
        requireNotNull(played)
        return played
    }

    private fun chunk(samples: Int): PcmChunk {
        val pcm = ShortArray(samples) { 3 }
        return PcmChunk(pcm, 48_000, 2)
    }
}
