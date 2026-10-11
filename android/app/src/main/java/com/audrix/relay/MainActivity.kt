package com.audrix.relay

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.widget.Button
import android.widget.EditText
import android.widget.TextView

class MainActivity : Activity() {
    private lateinit var status: TextView
    private lateinit var pcIp: EditText
    private val tick = Handler(Looper.getMainLooper())
    private val refresh = object : Runnable {
        override fun run() {
            status.text = RelayService.status
            tick.postDelayed(this, 500)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        status = findViewById(R.id.status)
        pcIp = findViewById(R.id.pc_ip)
        pcIp.setText(getSharedPreferences(PREFS, MODE_PRIVATE).getString(KEY_IP, ""))
        findViewById<Button>(R.id.connect).setOnClickListener { connect() }
    }

    override fun onStart() {
        super.onStart()
        tick.post(refresh)
    }

    override fun onStop() {
        tick.removeCallbacks(refresh)
        super.onStop()
    }

    private fun connect() {
        val ip = pcIp.text.toString().trim()
        if (ip.isEmpty()) {
            status.text = "ใส่ IP ของ PC ก่อน"
            return
        }
        getSharedPreferences(PREFS, MODE_PRIVATE).edit().putString(KEY_IP, ip).apply()
        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), REQUEST_NOTIFY)
            return
        }
        startRelay(ip)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == REQUEST_NOTIFY) {
            startRelay(pcIp.text.toString().trim())
        }
    }

    private fun startRelay(ip: String) {
        val intent = Intent(this, RelayService::class.java).putExtra(RelayService.EXTRA_IP, ip)
        if (Build.VERSION.SDK_INT >= 26) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }
    }

    private companion object {
        const val PREFS = "audrix"
        const val KEY_IP = "pc_ip"
        const val REQUEST_NOTIFY = 1
    }
}
