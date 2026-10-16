package com.audrix.relay

import org.junit.Assert.assertFalse
import org.junit.Test

class PlaybackSessionTest {
    @Test
    fun stopEndsPlaybackWithoutASocket() {
        val session = PlaybackSession()
        session.offer(0u, ShortArray(3840) { 1 }, 48_000, 2)
        session.stop()
        assertFalse(session.isActive())
    }

    @Test
    fun offerAndStopFromTwoThreadsDoNotThrow() {
        val session = PlaybackSession()
        var failure: Throwable? = null
        val threads = listOf(48_000, 44_100).map { rate ->
            Thread {
                try {
                    repeat(5_000) { seq ->
                        session.offer(seq.toUInt(), ShortArray(rate / 100 * 2), rate, 2)
                        if (seq % 50 == 0) session.stop()
                    }
                } catch (err: Throwable) {
                    failure = err
                }
            }
        }
        threads.forEach { it.start() }
        threads.forEach { it.join() }
        failure?.let { throw it }
    }
}
