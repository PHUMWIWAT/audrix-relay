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
}
