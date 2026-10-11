package com.audrix.relay

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Typeface
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.ListView
import android.widget.TextView

class MainActivity : Activity() {
    private lateinit var status: TextView
    private lateinit var latency: TextView
    private lateinit var ms: TextView
    private lateinit var pcs: ListView
    private lateinit var adapter: PeerAdapter
    private var shown = ""
    private val tick = Handler(Looper.getMainLooper())
    private val refresh = object : Runnable {
        override fun run() {
            showPeers()
            tick.postDelayed(this, 500)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        val serif = Typeface.create("serif", Typeface.NORMAL)
        findViewById<TextView>(R.id.title).typeface = serif
        latency = findViewById(R.id.latency)
        ms = findViewById(R.id.ms)
        latency.typeface = serif
        ms.typeface = serif
        status = findViewById(R.id.status)
        pcs = findViewById(R.id.pcs)
        adapter = PeerAdapter()
        pcs.adapter = adapter
        findViewById<Button>(R.id.disconnect).setOnClickListener {
            startService(Intent(this, RelayService::class.java).setAction(RelayService.ACTION_DISCONNECT))
        }
        pcs.setOnItemClickListener { _, _, position, _ ->
            val peer = adapter.getItem(position) ?: return@setOnItemClickListener
            val intent = Intent(this, RelayService::class.java).putExtra(RelayService.EXTRA_IP, peer.ip)
            if (Build.VERSION.SDK_INT >= 33 &&
                checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
            ) {
                pendingIp = peer.ip
                requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 1)
                return@setOnItemClickListener
            }
            startForegroundService(intent)
        }
        startForegroundService(Intent(this, RelayService::class.java))
    }

    override fun onStart() {
        super.onStart()
        tick.post(refresh)
    }

    override fun onStop() {
        tick.removeCallbacks(refresh)
        super.onStop()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        val ip = pendingIp ?: return
        pendingIp = null
        startForegroundService(Intent(this, RelayService::class.java).putExtra(RelayService.EXTRA_IP, ip))
    }

    private fun showPeers() {
        val peers = RelayService.peers()
        val signature = peers.joinToString { it.name + it.ip }
        if (signature != shown) {
            adapter.clear()
            adapter.addAll(peers)
            shown = signature
        }
        val delay = RelayService.delayMs
        val connected = RelayService.connectedIp
        findViewById<Button>(R.id.disconnect).visibility = if (connected != null) View.VISIBLE else View.GONE
        if (delay != null && connected != null) {
            latency.text = delay.toString()
            latency.visibility = View.VISIBLE
            ms.visibility = View.VISIBLE
            status.text = "กำลังเล่นจาก $connected"
        } else if (peers.isEmpty()) {
            latency.visibility = View.GONE
            ms.visibility = View.GONE
            status.text = "ยังไม่เห็นเครื่องในวงนี้ เปิดโปรแกรมบน PC ก่อน"
        } else {
            latency.visibility = View.GONE
            ms.visibility = View.GONE
            status.text = "แตะชื่อเครื่องเพื่อฟังเสียง"
        }
    }

    private inner class PeerAdapter : ArrayAdapter<RelayService.Peer>(this, 0) {
        override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
            val row = convertView ?: layoutInflater.inflate(R.layout.peer_row, parent, false)
            val peer = getItem(position) ?: return row
            row.findViewById<TextView>(R.id.name).text = peer.name
            row.findViewById<TextView>(R.id.ip).text = peer.ip
            val chosen = peer.ip == RelayService.connectedIp
            row.setBackgroundColor(if (chosen) 0xFF241816.toInt() else 0xFF1A1714.toInt())
            return row
        }
    }

    private var pendingIp: String? = null
}
