package com.audrix.relay

object ReconnectBackoff {
    const val INITIAL_DELAY_MS = 1_000L
    const val MAX_DELAY_MS = 30_000L

    fun delayMsFor(attempt: Int): Long {
        val shift = attempt.coerceIn(0, 5)
        return (INITIAL_DELAY_MS shl shift).coerceAtMost(MAX_DELAY_MS)
    }
}
