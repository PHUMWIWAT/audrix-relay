package com.audrix.relay

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.graphics.BitmapFactory
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.net.wifi.WifiManager
import android.os.Build
import android.os.SystemClock
import android.util.Log
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.SocketTimeoutException
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.charset.StandardCharsets

class RelayService : Service() {
    private var worker: Thread? = null
    @Volatile private var running = false
    @Volatile private var socket: DatagramSocket? = null
    private val playback = PlaybackSession()
    private var player: Thread? = null
    private var focus: AudioFocusRequest? = null
    private var wifiLock: WifiManager.WifiLock? = null
    @Volatile private var host: String = ""

    override fun onBind(intent: Intent?) = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP || intent?.action == ACTION_DISCONNECT) {
            host = ""
            connectedIp = null
            delayMs = null
            releaseWifi()
            val note = notification(Copy.searching())
            getSystemService(NotificationManager::class.java).notify(NOTE_ID, note)
            return START_STICKY
        }
        val ip = intent?.getStringExtra(EXTRA_IP).orEmpty().trim()
        if (ip.isNotEmpty()) {
            host = ip
            connectedIp = ip
            requestFocus()
            holdWifi()
        } else {
            releaseWifi()
        }
        val note = notification(if (host.isEmpty()) Copy.searching() else Copy.connecting())
        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(NOTE_ID, note, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
        } else {
            startForeground(NOTE_ID, note)
        }
        if (worker?.isAlive != true) {
            running = true
            player = Thread({ playLoop() }, "relay-play")
            player?.start()
            worker = Thread({
                var attempt = 0
                while (running) {
                    try {
                        listen()
                        break
                    } catch (err: Exception) {
                        if (!running || host.isEmpty()) {
                            break
                        }
                        status = "เชื่อมใหม่\n${err.message}"
                        Thread.sleep(ReconnectBackoff.delayMsFor(attempt))
                        attempt += 1
                    }
                }
            }, "relay-udp")
            worker?.start()
        }
        return START_STICKY
    }

    override fun onDestroy() {
        running = false
        socket?.close()
        worker?.join(1500)
        player?.join(1500)
        releaseTrack()
        releaseWifi()
        focus?.let { getSystemService(AudioManager::class.java).abandonAudioFocusRequest(it) }
        super.onDestroy()
    }

    private fun listen() {
        val bound = DatagramSocket()
        socket = bound
        bound.broadcast = true
        bound.soTimeout = 400
        val broadcast = InetAddress.getByName("255.255.255.255")
        var nextFind = 0L
        var nextHello = 0L
        var pingSentAt = 0L
        val loss = LossCount()
        val buffer = ByteArray(65536)
        val packet = DatagramPacket(buffer, buffer.size)
        var lastNote = 0L
        try {
            while (running) {
                if (host.isEmpty() && playback.isActive()) {
                    releaseTrack()
                }
                val now = SystemClock.elapsedRealtime()
                if (now >= nextFind) {
                    val find = "FIND".toByteArray(StandardCharsets.US_ASCII)
                    bound.send(DatagramPacket(find, find.size, broadcast, PORT))
                    prunePeers(now)
                    nextFind = now + 1000
                }
                val target = host
                if (target.isNotEmpty() && now >= nextHello) {
                    val remote = InetAddress.getByName(target)
                    val hello = ByteBuffer.allocate(if (delayMs == null) 4 else 6).order(ByteOrder.LITTLE_ENDIAN)
                    hello.put(HELLO)
                    if (delayMs != null) {
                        hello.putShort(delayMs!!.coerceIn(0, 65535).toShort())
                    }
                    val helloBytes = hello.array()
                    bound.send(DatagramPacket(helloBytes, helloBytes.size, remote, PORT))
                    val ping = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN)
                    ping.put("PING".toByteArray(StandardCharsets.US_ASCII))
                    ping.putInt(now.toInt())
                    val pingBytes = ping.array()
                    bound.send(DatagramPacket(pingBytes, pingBytes.size, remote, PORT))
                    pingSentAt = now
                    nextHello = now + 1000
                }
                val size = try {
                    bound.receive(packet)
                    packet.length
                } catch (_: SocketTimeoutException) {
                    continue
                } catch (err: Exception) {
                    if (running && host.isNotEmpty()) {
                        throw err
                    }
                    break
                }
                if (size >= 4 && buffer.copyOfRange(0, 4).contentEquals("HERE".toByteArray())) {
                    val name = String(buffer, 4, size - 4, StandardCharsets.UTF_8).trim().ifEmpty { "PC" }
                    val ip = packet.address?.hostAddress ?: continue
                    rememberPeer(name, ip, now)
                    continue
                }
                if (size == 8 && buffer.copyOfRange(0, 4).contentEquals("PONG".toByteArray())) {
                    val token = ByteBuffer.wrap(buffer, 4, 4).order(ByteOrder.LITTLE_ENDIAN).int
                    val arrived = SystemClock.elapsedRealtime()
                    if (token == pingSentAt.toInt()) {
                        val elapsed = (arrived - pingSentAt).toInt()
                        if (elapsed in 0..5000) {
                            delayMs = elapsed
                        }
                    }
                    continue
                }
                if (host.isEmpty() && playback.isActive()) {
                    releaseTrack()
                }
                val parsed = parsePacket(buffer, size) ?: continue
                if (host.isEmpty()) {
                    continue
                }
                loss.observe(parsed.seq)
                playback.offer(parsed.seq, parsed.pcm, parsed.sampleRate, parsed.channels)
                status = "${Copy.playingShort()}\n${parsed.sampleRate} Hz"
                if (now - lastNote > 1000) {
                    val delay = delayMs?.let { "$it ms" } ?: Copy.playingShort()
                    getSystemService(NotificationManager::class.java).notify(NOTE_ID, notification(delay))
                    lastNote = now
                }
            }
        } finally {
            bound.close()
            releaseTrack()
            connectedIp = null
            delayMs = null
        }
    }

    private fun rememberPeer(name: String, ip: String, now: Long) {
        synchronized(knownPeers) {
            val existing = knownPeers.find { it.ip == ip }
            if (existing == null) {
                knownPeers.add(Peer(name, ip, now))
            } else {
                existing.name = name
                existing.seen = now
            }
        }
    }

    private fun prunePeers(now: Long) {
        synchronized(knownPeers) {
            knownPeers.removeAll { now - it.seen > 4000 }
        }
    }

    private fun playLoop() {
        while (running) {
            // One bad chunk must not end the thread: the UI would still show
            // the PC as connected while nothing plays.
            try {
                if (host.isEmpty()) {
                    playback.stop()
                    Thread.sleep(20)
                    continue
                }
                when (playback.playNext()) {
                    null -> Thread.sleep(5)
                    PlaybackTrack.WriteResult.FAILED -> Thread.sleep(20)
                    else -> {}
                }
            } catch (err: Exception) {
                Log.w("RelayService", "playback step failed", err)
                runCatching { Thread.sleep(20) }
            }
        }
    }

    private fun holdWifi() {
        if (wifiLock?.isHeld == true) {
            return
        }
        val wifi = applicationContext.getSystemService(WifiManager::class.java) ?: return
        val mode = if (Build.VERSION.SDK_INT >= 29) {
            WifiManager.WIFI_MODE_FULL_LOW_LATENCY
        } else {
            @Suppress("DEPRECATION")
            WifiManager.WIFI_MODE_FULL_HIGH_PERF
        }
        wifiLock = wifi.createWifiLock(mode, "audrix-relay").apply {
            setReferenceCounted(false)
            acquire()
        }
    }

    private fun releaseWifi() {
        wifiLock?.let { lock ->
            if (lock.isHeld) {
                lock.release()
            }
        }
        wifiLock = null
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
        playback.stop()
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
            .setLargeIcon(BitmapFactory.decodeResource(resources, R.drawable.logo_mark))
            .setContentTitle("AUDRIX RELAY")
            .setContentText(text)
            .setOngoing(true)
            .addAction(Notification.Action.Builder(null, Copy.stop(), stop).build())
            .build()
    }

    class Peer(var name: String, val ip: String, var seen: Long)

    companion object {
        const val EXTRA_IP = "pc_ip"
        const val ACTION_STOP = "stop"
        const val ACTION_DISCONNECT = "disconnect"
        const val PORT = 40123
        private const val CHANNEL = "relay"
        private const val NOTE_ID = 1
        private val HELLO = byteArrayOf(0x41, 0x58, 0x44, 0x52)
        private val knownPeers = mutableListOf<Peer>()

        @Volatile
        var status: String = "แตะชื่อเครื่องเพื่อฟังเสียง"

        @Volatile
        var connectedIp: String? = null

        @Volatile
        var delayMs: Int? = null

        fun peers(): List<Peer> {
            synchronized(knownPeers) {
                return knownPeers.map { Peer(it.name, it.ip, it.seen) }
            }
        }
    }
}
