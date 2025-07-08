//
//  AudioMonCollector.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import AVFoundation
import CoreAudio
import AppKit
import os.log

/// Audio activity monitoring collector
public class AudioMonCollector: CollectorBase {
    private let permissionManager: PermissionManager
    private var monitoringTimer: Timer?
    private var audioEngine: AVAudioEngine?
    private var isMonitoring = false
    
    public init(configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter,
                permissionManager: PermissionManager = PermissionManager()) {
        self.permissionManager = permissionManager
        
        super.init(
            identifier: "audio_mon",
            displayName: "Audio Monitor",
            eventTypes: [.audioActivity],
            configuration: configuration,
            ringBufferWriter: ringBufferWriter
        )
    }
    
    // MARK: - CollectorProtocol Implementation
    
    public override func checkPermissions() -> Bool {
        return permissionManager.checkPermission(.microphone) == .granted
    }
    
    public override func requestPermissions() async throws {
        try await permissionManager.requestPermission(.microphone)
    }
    
    public override func startCollector() throws {
        guard checkPermissions() else {
            throw ChronicleCollectorError.permissionDenied("Microphone permission required")
        }
        
        startMonitoring()
        isMonitoring = true
        
        logger.info("Audio monitor collector started successfully")
    }
    
    public override func stopCollector() throws {
        stopMonitoring()
        isMonitoring = false
        
        logger.info("Audio monitor collector stopped")
    }
    
    // MARK: - Monitoring
    
    private func startMonitoring() {
        let interval = 1.0 / currentFrameRate
        
        monitoringTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            self?.checkAudioActivity()
        }
    }
    
    private func stopMonitoring() {
        monitoringTimer?.invalidate()
        monitoringTimer = nil
        
        audioEngine?.stop()
        audioEngine = nil
    }
    
    private func checkAudioActivity() {
        guard isRunning && isMonitoring else { return }
        
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        // Get audio system status
        let audioStatus = getAudioSystemStatus()
        
        // Detect meeting applications
        let meetingDetected = detectMeetingApplications()
        
        // Create audio activity event
        let eventData = AudioActivityEventData(
            isInputActive: audioStatus.isInputActive,
            isOutputActive: audioStatus.isOutputActive,
            inputLevel: audioStatus.inputLevel,
            outputLevel: audioStatus.outputLevel,
            isMicrophoneMuted: audioStatus.isMicrophoneMuted,
            isSystemMuted: audioStatus.isSystemMuted,
            activeApplications: audioStatus.activeApplications,
            meetingDetected: meetingDetected
        )
        
        emitAudioEvent(eventData)
        updateActivity()
    }
    
    private func emitAudioEvent(_ eventData: AudioActivityEventData) {
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            let chronicleEvent = createEvent(type: .audioActivity, data: jsonData, metadata: [
                "is_input_active": String(eventData.isInputActive),
                "is_output_active": String(eventData.isOutputActive),
                "meeting_detected": String(eventData.meetingDetected),
                "active_app_count": String(eventData.activeApplications.count)
            ])
            
            emitEvent(chronicleEvent)
            
        } catch {
            logger.error("Failed to encode audio event: \(error)")
        }
    }
    
    // MARK: - Audio System Status
    
    private struct AudioSystemStatus {
        let isInputActive: Bool
        let isOutputActive: Bool
        let inputLevel: Double
        let outputLevel: Double
        let isMicrophoneMuted: Bool
        let isSystemMuted: Bool
        let activeApplications: [String]
    }
    
    private func getAudioSystemStatus() -> AudioSystemStatus {
        // Get default audio devices
        let inputDevice = getDefaultAudioDevice(isInput: true)
        let outputDevice = getDefaultAudioDevice(isInput: false)
        
        // Check if audio is active (simplified implementation)
        let isInputActive = inputDevice != nil && !isMicrophoneMuted()
        let isOutputActive = outputDevice != nil && !isSystemMuted()
        
        // Get audio levels (simplified - would need real-time audio analysis)
        let inputLevel = isInputActive ? Double.random(in: 0...1) : 0.0
        let outputLevel = isOutputActive ? Double.random(in: 0...1) : 0.0
        
        // Get applications using audio
        let activeApplications = getApplicationsUsingAudio()
        
        return AudioSystemStatus(
            isInputActive: isInputActive,
            isOutputActive: isOutputActive,
            inputLevel: inputLevel,
            outputLevel: outputLevel,
            isMicrophoneMuted: isMicrophoneMuted(),
            isSystemMuted: isSystemMuted(),
            activeApplications: activeApplications
        )
    }
    
    private func getDefaultAudioDevice(isInput: Bool) -> AudioDeviceID? {
        var deviceID: AudioDeviceID = 0
        var size = UInt32(MemoryLayout<AudioDeviceID>.size)
        
        let property = isInput ? kAudioHardwarePropertyDefaultInputDevice : kAudioHardwarePropertyDefaultOutputDevice
        
        let status = AudioHardwareGetProperty(property, &size, &deviceID)
        
        return status == noErr ? deviceID : nil
    }
    
    private func isMicrophoneMuted() -> Bool {
        // Simplified check - in real implementation would check actual mute status
        return false
    }
    
    private func isSystemMuted() -> Bool {
        // Simplified check - in real implementation would check system volume
        return false
    }
    
    private func getApplicationsUsingAudio() -> [String] {
        // Get running applications that might be using audio
        let runningApps = NSWorkspace.shared.runningApplications
        let audioApps = runningApps.compactMap { app -> String? in
            guard let bundleId = app.bundleIdentifier else { return nil }
            
            // Check known audio applications
            let audioAppBundles = [
                "com.apple.music",
                "com.spotify.client",
                "com.microsoft.teams",
                "us.zoom.xos",
                "com.skype.skype",
                "com.apple.facetime",
                "com.google.chrome",
                "org.mozilla.firefox",
                "com.apple.safari"
            ]
            
            return audioAppBundles.contains(bundleId) ? (app.localizedName ?? bundleId) : nil
        }
        
        return audioApps
    }
    
    // MARK: - Meeting Detection
    
    private func detectMeetingApplications() -> Bool {
        let runningApps = NSWorkspace.shared.runningApplications
        
        // Known meeting application bundle identifiers
        let meetingApps = [
            "us.zoom.xos",
            "com.microsoft.teams",
            "com.skype.skype",
            "com.apple.facetime",
            "com.google.meet",
            "com.webex.meetingmanager",
            "com.gotomeeting.GoToMeetingWinStore",
            "com.bluejeans.mac"
        ]
        
        return runningApps.contains { app in
            guard let bundleId = app.bundleIdentifier else { return false }
            return meetingApps.contains(bundleId)
        }
    }
    
    // MARK: - Utility Methods
    
    private var currentFrameRate: Double {
        return configuration.adaptiveFrameRate ? adaptiveFrameRate : configuration.activeFrameRate
    }
    
    private var adaptiveFrameRate: Double {
        // Audio monitoring can be less frequent when no activity
        return configuration.idleFrameRate
    }
    
    /// Get audio system information
    public func getAudioSystemInfo() -> [String: Any] {
        let status = getAudioSystemStatus()
        
        return [
            "is_monitoring": isMonitoring,
            "input_device": getDefaultAudioDevice(isInput: true) ?? 0,
            "output_device": getDefaultAudioDevice(isInput: false) ?? 0,
            "is_input_active": status.isInputActive,
            "is_output_active": status.isOutputActive,
            "input_level": status.inputLevel,
            "output_level": status.outputLevel,
            "active_applications": status.activeApplications,
            "meeting_detected": detectMeetingApplications()
        ]
    }
}