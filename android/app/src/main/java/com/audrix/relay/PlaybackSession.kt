package com.audrix.relay

/**
 * Owns the jitter buffer and the playback track. Callers hand in PCM and
 * ask it to play the next chunk. They do not build either object.
 *
 * The receive thread calls [offer] while the play thread calls [playNext],
 * so all state is guarded by this object's lock. Only the blocking
 * `AudioTrack.write` runs outside it.
 */
class PlaybackSession {
    private var jitter: JitterBuffer? = null
    private var playback: PlaybackTrack? = null
    private var rate = 0
    private var channels = 0
    private var stopped = false

    @Synchronized
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
        val track: PlaybackTrack
        val chunk: ByteArray
        synchronized(this) {
            if (stopped) {
                return null
            }
            chunk = jitter?.pop() ?: return null
            track = playback ?: PlaybackTrack(rate, channels).also {
                it.play()
                playback = it
            }
        }
        // A stop during this write releases the track, which unblocks it.
        return track.write(chunk)
    }

    @Synchronized
    fun stop() {
        stopped = true
        stopPlayback()
    }

    @Synchronized
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
