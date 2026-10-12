package com.audrix.relay

import org.junit.Assert.assertEquals
import org.junit.Test

class ReconnectBackoffTest {
    @Test
    fun doublesUntilThirtySeconds() {
        assertEquals(1_000L, ReconnectBackoff.delayMsFor(0))
        assertEquals(2_000L, ReconnectBackoff.delayMsFor(1))
        assertEquals(30_000L, ReconnectBackoff.delayMsFor(8))
    }
}
