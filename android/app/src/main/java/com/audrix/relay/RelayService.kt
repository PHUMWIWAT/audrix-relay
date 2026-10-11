package com.audrix.relay

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.os.Build
import android.os.SystemClock
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.SocketTimeoutException

class RelayService : Service() {
    private var worker: Thread? = null
    @Volatile private var running = false
    @Volatile private var socket: DatagramSocket? = null
    private var track: AudioTrack? = null
    private var trackRate = 0
    private var trackChannels = 0
    private var focus: AudioFocusRequest? = null

    override fun onBind(intent: Intent?) = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            stopSelf()
            return START_NOT_STICKY
        }
        val ip = intent?.getStringExtra(EXTRA_IP).orEmpty().trim()
        val note = notification("กำลังเชื่อม $ip")
        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(NOTE_ID, note, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
        } else {
            startForeground(NOTE_ID, note)
        }
        running = false
        socket?.close()
        worker?.join(1000)
        if (ip.isEmpty()) {
            status = "ใส่ IP ของ PC ก่อน"
            stopSelf()
            return START_NOT_STICKY
        }
        running = true
        worker = Thread({ runCatching { listen(ip) }.onFailure { status = "เชื่อมไม่ได้\n${it.message}" } }, "relay-udp")
        worker?.start()
        return START_STICKY
    }

    override fun onDestroy() {
        running = false
        socket?.close()
        worker?.join(1500)
        releaseTrack()
        focus?.let { getSystemService(AudioManager::class.java).abandonAudioFocusRequest(it) }
        super.onDestroy()
    }

    private fun listen(host: String) {
        val remote = InetAddress.getByName(host)
        val bound = DatagramSocket()
        socket = bound
        bound.soTimeout = 400
        requestFocus()
        val hello = HELLO.copyOf()
        var nextHello = 0L
        val loss = LossCount()
        val buffer = ByteArray(65536)
        val packet = DatagramPacket(buffer, buffer.size)
        var lastNote = 0L
        try {
            while (running) {
                val now = SystemClock.elapsedRealtime()
                if (now >= nextHello) {
                    bound.send(DatagramPacket(hello, hello.size, remote, PORT))
                    nextHello = now + 2000
                }
                val size = try {
                    bound.receive(packet)
                    packet.length
                } catch (_: SocketTimeoutException) {
                    continue
                } catch (_: Exception) {
                    break
                }
                val parsed = parsePacket(buffer, size) ?: continue
                loss.observe(parsed.seq)
                val played = play(parsed)
                status = "เชื่อม $host\nแพ็กเก็ต ${loss.received}\nหาย ${loss.lost}\n${parsed.sampleRate} Hz · ${parsed.channels} ช่อง\n$played\nย่อแอปได้ เสียงยังเล่น"
                if (now - lastNote > 1000) {
                    getSystemService(NotificationManager::class.java).notify(NOTE_ID, notification(played))
                    lastNote = now
                }
            }
        } finally {
            bound.close()
            releaseTrack()
        }
    }

    private fun play(parsed: PcmPacket): String {
        val mask = when (parsed.channels) {
            1 -> AudioFormat.CHANNEL_OUT_MONO
            2 -> AudioFormat.CHANNEL_OUT_STEREO
            else -> return "ยังไม่เล่น: รองรับ 1 หรือ 2 ช่อง"
        }
        if (track == null || trackRate != parsed.sampleRate || trackChannels != parsed.channels) {
            releaseTrack()
            val min = AudioTrack.getMinBufferSize(parsed.sampleRate, mask, AudioFormat.ENCODING_PCM_16BIT)
            if (min <= 0) {
                return "ยังไม่เล่น: เครื่องไม่รับรูปแบบนี้"
            }
            track = AudioTrack.Builder()
                .setAudioAttributes(mediaAttributes())
                .setAudioFormat(
                    AudioFormat.Builder()
                        .setSampleRate(parsed.sampleRate)
                        .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                        .setChannelMask(mask)
                        .build(),
                )
                .setBufferSizeInBytes(min * 2)
                .setTransferMode(AudioTrack.MODE_STREAM)
                .build()
            track?.play()
            trackRate = parsed.sampleRate
            trackChannels = parsed.channels
        }
        val written = track?.write(parsed.pcm, 0, parsed.pcm.size) ?: return "ยังไม่เล่น"
        if (written < 0) {
            return "เขียนเสียงไม่ได้"
        }
        return "กำลังเล่น"
    }

    private fun requestFocus() {
        val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
            .setAudioAttributes(mediaAttributes())
            .build()
        focus = request
        getSystemService(AudioManager::class.java).requestAudioFocus(request)
    }

    private fun mediaAttributes(): AudioAttributes {
        return AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
            .build()
    }

    private fun releaseTrack() {
        val current = track ?: return
        track = null
        trackRate = 0
        trackChannels = 0
        try {
            current.pause()
        } catch (_: IllegalStateException) {
        }
        current.release()
    }

    private fun notification(text: String): Notification {
        val manager = getSystemService(NotificationManager::class.java)
        if (Build.VERSION.SDK_INT >= 26 && manager.getNotificationChannel(CHANNEL) == null) {
            manager.createNotificationChannel(
                NotificationChannel(CHANNEL, "เล่นเสียง", NotificationManager.IMPORTANCE_LOW),
            )
        }
        val stop = PendingIntent.getService(
            this,
            0,
            Intent(this, RelayService::class.java).setAction(ACTION_STOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        return Notification.Builder(this, CHANNEL)
            .setSmallIcon(android.R.drawable.ic_media_play)
            .setContentTitle("AUDRIX RELAY")
            .setContentText(text)
            .setOngoing(true)
            .addAction(Notification.Action.Builder(null, "หยุด", stop).build())
            .build()
    }

    companion object {
        const val EXTRA_IP = "pc_ip"
        const val ACTION_STOP = "stop"
        const val PORT = 40123
        private const val CHANNEL = "relay"
        private const val NOTE_ID = 1
        private val HELLO = byteArrayOf(0x41, 0x58, 0x44, 0x52)

        @Volatile
        var status: String = "ใส่ IP ที่หน้าต่าง PC แสดง แล้วกดเชื่อมต่อ"
    }
}
