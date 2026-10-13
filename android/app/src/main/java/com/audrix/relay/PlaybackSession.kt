package com.audrix.relay

/**
 * Owns the jitter buffer and the playback track. Callers hand in PCM and
 * ask it to play the next chunk. They do not build either object.
 */
class PlaybackSession {
    private var jitter: JitterBuffer? = null
    private var playback: PlaybackTrack? = null
    private var rate = 0
    private var channels = 0
    private var stopped = false

    fun offer(seq: UInt, samples: ShortArray, sampleRate: Int, channelCount: Int) {
        if (sampleRate <= 0 || channelCount <= 0) {
            return
        }
        stopped = false
        if (jitter == null || rate != sampleRate || channels != channelCount) {
            stopPlayback()
            val bytesPerFrame = channelCount * 2
            jitter = JitterBuffer(
                chunkSizeBytes = sampleRate / 100 * bytesPerFrame,
                bytesPerFrame = bytesPerFrame,
                targetDepthMs = 80,
                bytesPerMs = sampleRate * bytesPerFrame / 1000,
            )
            rate = sampleRate
            channels = channelCount
        }
        jitter?.push(seq, soften(samples))
    }

    fun playNext(): PlaybackTrack.WriteResult? {
        if (stopped) {
            return null
        }
        val chunk = jitter?.pop() ?: return null
        if (playback == null) {
            playback = PlaybackTrack(rate, channels).also { it.play() }
        }
        return playback?.write(chunk)
    }

    fun stop() {
        stopped = true
        stopPlayback()
    }

    fun isActive(): Boolean = !stopped && jitter != null

    private fun stopPlayback() {
        jitter = null
        playback?.shutdown()
        playback = null
        rate = 0
        channels = 0
    }

    private fun soften(samples: ShortArray): ByteArray {
        val out = ByteArray(samples.size * 2)
        for (i in samples.indices) {
            val value = (samples[i] * SPEAKER_GAIN).toInt().toShort().toInt()
            out[i * 2] = (value and 0xff).toByte()
            out[i * 2 + 1] = (value shr 8 and 0xff).toByte()
        }
        return out
    }

    private companion object {
        const val SPEAKER_GAIN = 0.71f
    }
}
