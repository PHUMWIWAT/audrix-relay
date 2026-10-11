package com.audrix.relay

import android.app.Activity
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Bundle
import android.widget.TextView
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.Inet4Address
import java.net.NetworkInterface
import java.net.SocketTimeoutException

class MainActivity : Activity() {
    private lateinit var status: TextView
    private var worker: Thread? = null
    @Volatile private var running = false
    @Volatile private var socket: DatagramSocket? = null
    private var track: AudioTrack? = null
    private var trackRate = 0
    private var trackChannels = 0

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        status = findViewById(R.id.status)
        showIdle()
    }

    override fun onStart() {
        super.onStart()
        running = true
        worker = Thread(::listen, "relay-udp").also { it.start() }
    }

    override fun onStop() {
        running = false
        socket?.close()
        worker?.join(2000)
        worker = null
        super.onStop()
    }

    private fun listen() {
        val bound = try {
            DatagramSocket(PORT)
        } catch (err: Exception) {
            show("เปิดพอร์ต $PORT ไม่ได้\n${err.message}")
            return
        }
        socket = bound
        bound.soTimeout = 500
        val loss = LossCount()
        val buffer = ByteArray(65536)
        val packet = DatagramPacket(buffer, buffer.size)
        try {
            while (running) {
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
                show(
                    "ให้ PC ส่งมาที่ ${ipv4()}\n" +
                        "พอร์ต $PORT\n" +
                        "แพ็กเก็ต ${loss.received}\n" +
                        "หาย ${loss.lost}\n" +
                        "${parsed.sampleRate} Hz · ${parsed.channels} ช่อง\n" +
                        played,
                )
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
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build(),
                )
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

    private fun showIdle() {
        status.text = "ให้ PC ส่งมาที่ ${ipv4()}\nพอร์ต $PORT"
    }

    private fun show(text: String) {
        runOnUiThread { status.text = text }
    }

    private fun ipv4(): String {
        val interfaces = NetworkInterface.getNetworkInterfaces() ?: return "ไม่พบ IP"
        for (iface in interfaces) {
            if (!iface.isUp || iface.isLoopback) {
                continue
            }
            for (addr in iface.inetAddresses) {
                if (addr is Inet4Address) {
                    return addr.hostAddress ?: continue
                }
            }
        }
        return "ไม่พบ IP"
    }

    private companion object {
        const val PORT = 40123
    }
}
