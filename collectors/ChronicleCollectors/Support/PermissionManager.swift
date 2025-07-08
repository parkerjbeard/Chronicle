//
//  PermissionManager.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import AVFoundation
import Contacts
import EventKit
import Photos
import CoreLocation
import Speech
import AppKit
import os.log

/// Permission types required by Chronicle collectors
public enum PermissionType: String, CaseIterable {
    case accessibility = "accessibility"
    case screenRecording = "screen_recording"
    case inputMonitoring = "input_monitoring"
    case microphone = "microphone"
    case camera = "camera"
    case contacts = "contacts"
    case calendars = "calendars"
    case reminders = "reminders"
    case photos = "photos"
    case speechRecognition = "speech_recognition"
    case fileAccess = "file_access"
    case fullDiskAccess = "full_disk_access"
    case systemPolicyControl = "system_policy_control"
    
    public var displayName: String {
        switch self {
        case .accessibility:
            return "Accessibility"
        case .screenRecording:
            return "Screen Recording"
        case .inputMonitoring:
            return "Input Monitoring"
        case .microphone:
            return "Microphone"
        case .camera:
            return "Camera"
        case .contacts:
            return "Contacts"
        case .calendars:
            return "Calendars"
        case .reminders:
            return "Reminders"
        case .photos:
            return "Photos"
        case .speechRecognition:
            return "Speech Recognition"
        case .fileAccess:
            return "File Access"
        case .fullDiskAccess:
            return "Full Disk Access"
        case .systemPolicyControl:
            return "System Policy Control"
        }
    }
    
    public var description: String {
        switch self {
        case .accessibility:
            return "Required to monitor keyboard and mouse events"
        case .screenRecording:
            return "Required to capture screen content"
        case .inputMonitoring:
            return "Required to monitor keyboard and mouse input"
        case .microphone:
            return "Required to detect audio activity and meetings"
        case .camera:
            return "Required to detect camera usage"
        case .contacts:
            return "Required to enrich meeting and communication data"
        case .calendars:
            return "Required to correlate activity with scheduled events"
        case .reminders:
            return "Required to correlate activity with tasks"
        case .photos:
            return "Required to monitor screenshot and image activity"
        case .speechRecognition:
            return "Required to transcribe meetings and calls"
        case .fileAccess:
            return "Required to monitor file system activity"
        case .fullDiskAccess:
            return "Required to monitor system-wide file activity"
        case .systemPolicyControl:
            return "Required to monitor system-wide activity"
        }
    }
}

/// Permission status
public enum PermissionStatus {
    case notDetermined
    case granted
    case denied
    case restricted
    case unknown
}

/// Permission manager for handling macOS TCC permissions
public class PermissionManager: ObservableObject {
    private let logger = Logger(subsystem: "com.chronicle.collectors", category: "PermissionManager")
    
    @Published public private(set) var permissions: [PermissionType: PermissionStatus] = [:]
    
    public init() {
        updateAllPermissions()
    }
    
    /// Check if all required permissions are granted
    public func hasAllRequiredPermissions(for types: [PermissionType]) -> Bool {
        return types.allSatisfy { permissions[$0] == .granted }
    }
    
    /// Check specific permission status
    public func checkPermission(_ type: PermissionType) -> PermissionStatus {
        let status = getPermissionStatus(type)
        permissions[type] = status
        return status
    }
    
    /// Request specific permission
    public func requestPermission(_ type: PermissionType) async throws {
        logger.info("Requesting permission: \(type.displayName)")
        
        switch type {
        case .accessibility:
            try await requestAccessibilityPermission()
        case .screenRecording:
            try await requestScreenRecordingPermission()
        case .inputMonitoring:
            try await requestInputMonitoringPermission()
        case .microphone:
            try await requestMicrophonePermission()
        case .camera:
            try await requestCameraPermission()
        case .contacts:
            try await requestContactsPermission()
        case .calendars:
            try await requestCalendarsPermission()
        case .reminders:
            try await requestRemindersPermission()
        case .photos:
            try await requestPhotosPermission()
        case .speechRecognition:
            try await requestSpeechRecognitionPermission()
        case .fileAccess, .fullDiskAccess:
            try await requestFileAccessPermission()
        case .systemPolicyControl:
            try await requestSystemPolicyControlPermission()
        }
        
        // Update permission status
        permissions[type] = checkPermission(type)
    }
    
    /// Request multiple permissions
    public func requestPermissions(_ types: [PermissionType]) async throws {
        for type in types {
            try await requestPermission(type)
        }
    }
    
    /// Open system preferences for specific permission
    public func openSystemPreferences(for type: PermissionType) {
        let url: String
        
        switch type {
        case .accessibility:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        case .screenRecording:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        case .inputMonitoring:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        case .microphone:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        case .camera:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Camera"
        case .contacts:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Contacts"
        case .calendars:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars"
        case .reminders:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Reminders"
        case .photos:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Photos"
        case .speechRecognition:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_SpeechRecognition"
        case .fileAccess, .fullDiskAccess:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"
        case .systemPolicyControl:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_SystemPolicyControl"
        }
        
        if let systemPrefsURL = URL(string: url) {
            NSWorkspace.shared.open(systemPrefsURL)
        }
    }
    
    /// Update all permission statuses
    public func updateAllPermissions() {
        for type in PermissionType.allCases {
            permissions[type] = checkPermission(type)
        }
    }
    
    /// Get permission requirements for specific collectors
    public func getRequiredPermissions(for collectorIds: [String]) -> [PermissionType] {
        var required: Set<PermissionType> = []
        
        for collectorId in collectorIds {
            switch collectorId {
            case "key_tap":
                required.insert(.accessibility)
                required.insert(.inputMonitoring)
            case "screen_tap":
                required.insert(.screenRecording)
            case "window_mon":
                required.insert(.accessibility)
            case "pointer_mon":
                required.insert(.accessibility)
                required.insert(.inputMonitoring)
            case "clip_mon":
                required.insert(.accessibility)
            case "fs_mon":
                required.insert(.fileAccess)
                required.insert(.fullDiskAccess)
            case "audio_mon":
                required.insert(.microphone)
                required.insert(.camera)
            case "net_mon":
                required.insert(.systemPolicyControl)
            default:
                break
            }
        }
        
        return Array(required)
    }
    
    // MARK: - Private Methods
    
    private func getPermissionStatus(_ type: PermissionType) -> PermissionStatus {
        switch type {
        case .accessibility:
            return getAccessibilityPermissionStatus()
        case .screenRecording:
            return getScreenRecordingPermissionStatus()
        case .inputMonitoring:
            return getInputMonitoringPermissionStatus()
        case .microphone:
            return getMicrophonePermissionStatus()
        case .camera:
            return getCameraPermissionStatus()
        case .contacts:
            return getContactsPermissionStatus()
        case .calendars:
            return getCalendarsPermissionStatus()
        case .reminders:
            return getRemindersPermissionStatus()
        case .photos:
            return getPhotosPermissionStatus()
        case .speechRecognition:
            return getSpeechRecognitionPermissionStatus()
        case .fileAccess, .fullDiskAccess:
            return getFileAccessPermissionStatus()
        case .systemPolicyControl:
            return getSystemPolicyControlPermissionStatus()
        }
    }
    
    // MARK: - Permission Status Checks
    
    private func getAccessibilityPermissionStatus() -> PermissionStatus {
        let checkOptPrompt = kAXTrustedCheckOptionPrompt.takeUnretainedValue() as NSString
        let options = [checkOptPrompt: false]
        let accessEnabled = AXIsProcessTrustedWithOptions(options as CFDictionary)
        return accessEnabled ? .granted : .denied
    }
    
    private func getScreenRecordingPermissionStatus() -> PermissionStatus {
        if #available(macOS 10.15, *) {
            let stream = CGDisplayStream(
                display: CGMainDisplayID(),
                outputWidth: 1,
                outputHeight: 1,
                pixelFormat: Int32(kCVPixelFormatType_32BGRA),
                properties: nil,
                queue: DispatchQueue.global()
            ) { _, _, _, _ in }
            
            return stream != nil ? .granted : .denied
        }
        return .granted
    }
    
    private func getInputMonitoringPermissionStatus() -> PermissionStatus {
        // Check if we can create an event tap
        let eventTap = CGEvent.tapCreate(
            tap: .cgSessionEventTap,
            place: .headInsertEventTap,
            options: .defaultTap,
            eventsOfInterest: CGEventMask(1 << CGEventType.keyDown.rawValue),
            callback: { _, _, _, _ in nil },
            userInfo: nil
        )
        
        if eventTap != nil {
            CFRelease(eventTap)
            return .granted
        }
        
        return .denied
    }
    
    private func getMicrophonePermissionStatus() -> PermissionStatus {
        switch AVCaptureDevice.authorizationStatus(for: .audio) {
        case .authorized:
            return .granted
        case .denied:
            return .denied
        case .restricted:
            return .restricted
        case .notDetermined:
            return .notDetermined
        @unknown default:
            return .unknown
        }
    }
    
    private func getCameraPermissionStatus() -> PermissionStatus {
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized:
            return .granted
        case .denied:
            return .denied
        case .restricted:
            return .restricted
        case .notDetermined:
            return .notDetermined
        @unknown default:
            return .unknown
        }
    }
    
    private func getContactsPermissionStatus() -> PermissionStatus {
        switch CNContactStore.authorizationStatus(for: .contacts) {
        case .authorized:
            return .granted
        case .denied:
            return .denied
        case .restricted:
            return .restricted
        case .notDetermined:
            return .notDetermined
        @unknown default:
            return .unknown
        }
    }
    
    private func getCalendarsPermissionStatus() -> PermissionStatus {
        switch EKEventStore.authorizationStatus(for: .event) {
        case .authorized:
            return .granted
        case .denied:
            return .denied
        case .restricted:
            return .restricted
        case .notDetermined:
            return .notDetermined
        @unknown default:
            return .unknown
        }
    }
    
    private func getRemindersPermissionStatus() -> PermissionStatus {
        switch EKEventStore.authorizationStatus(for: .reminder) {
        case .authorized:
            return .granted
        case .denied:
            return .denied
        case .restricted:
            return .restricted
        case .notDetermined:
            return .notDetermined
        @unknown default:
            return .unknown
        }
    }
    
    private func getPhotosPermissionStatus() -> PermissionStatus {
        switch PHPhotoLibrary.authorizationStatus() {
        case .authorized:
            return .granted
        case .denied:
            return .denied
        case .restricted:
            return .restricted
        case .notDetermined:
            return .notDetermined
        case .limited:
            return .granted
        @unknown default:
            return .unknown
        }
    }
    
    private func getSpeechRecognitionPermissionStatus() -> PermissionStatus {
        switch SFSpeechRecognizer.authorizationStatus() {
        case .authorized:
            return .granted
        case .denied:
            return .denied
        case .restricted:
            return .restricted
        case .notDetermined:
            return .notDetermined
        @unknown default:
            return .unknown
        }
    }
    
    private func getFileAccessPermissionStatus() -> PermissionStatus {
        // Check if we can access sensitive directories
        let sensitiveDirectories = [
            "/System",
            "/Library",
            "/Applications",
            "/Users"
        ]
        
        for directory in sensitiveDirectories {
            let url = URL(fileURLWithPath: directory)
            if FileManager.default.isReadableFile(atPath: url.path) {
                return .granted
            }
        }
        
        return .denied
    }
    
    private func getSystemPolicyControlPermissionStatus() -> PermissionStatus {
        // This is harder to check programmatically
        // For now, assume it needs to be granted manually
        return .notDetermined
    }
    
    // MARK: - Permission Requests
    
    private func requestAccessibilityPermission() async throws {
        let checkOptPrompt = kAXTrustedCheckOptionPrompt.takeUnretainedValue() as NSString
        let options = [checkOptPrompt: true]
        let accessEnabled = AXIsProcessTrustedWithOptions(options as CFDictionary)
        
        if !accessEnabled {
            throw ChronicleCollectorError.permissionDenied("Accessibility permission required")
        }
    }
    
    private func requestScreenRecordingPermission() async throws {
        if #available(macOS 10.15, *) {
            // Screen recording permission is requested automatically when attempting to capture
            // We'll just check if it's available
            let status = getScreenRecordingPermissionStatus()
            if status != .granted {
                throw ChronicleCollectorError.permissionDenied("Screen recording permission required")
            }
        }
    }
    
    private func requestInputMonitoringPermission() async throws {
        // Input monitoring permission is requested automatically when creating event taps
        let status = getInputMonitoringPermissionStatus()
        if status != .granted {
            throw ChronicleCollectorError.permissionDenied("Input monitoring permission required")
        }
    }
    
    private func requestMicrophonePermission() async throws {
        return try await withCheckedThrowingContinuation { continuation in
            AVCaptureDevice.requestAccess(for: .audio) { granted in
                if granted {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: ChronicleCollectorError.permissionDenied("Microphone permission required"))
                }
            }
        }
    }
    
    private func requestCameraPermission() async throws {
        return try await withCheckedThrowingContinuation { continuation in
            AVCaptureDevice.requestAccess(for: .video) { granted in
                if granted {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: ChronicleCollectorError.permissionDenied("Camera permission required"))
                }
            }
        }
    }
    
    private func requestContactsPermission() async throws {
        return try await withCheckedThrowingContinuation { continuation in
            let store = CNContactStore()
            store.requestAccess(for: .contacts) { granted, error in
                if granted {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: error ?? ChronicleCollectorError.permissionDenied("Contacts permission required"))
                }
            }
        }
    }
    
    private func requestCalendarsPermission() async throws {
        return try await withCheckedThrowingContinuation { continuation in
            let store = EKEventStore()
            store.requestAccess(to: .event) { granted, error in
                if granted {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: error ?? ChronicleCollectorError.permissionDenied("Calendar permission required"))
                }
            }
        }
    }
    
    private func requestRemindersPermission() async throws {
        return try await withCheckedThrowingContinuation { continuation in
            let store = EKEventStore()
            store.requestAccess(to: .reminder) { granted, error in
                if granted {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: error ?? ChronicleCollectorError.permissionDenied("Reminders permission required"))
                }
            }
        }
    }
    
    private func requestPhotosPermission() async throws {
        return try await withCheckedThrowingContinuation { continuation in
            PHPhotoLibrary.requestAuthorization { status in
                switch status {
                case .authorized, .limited:
                    continuation.resume()
                default:
                    continuation.resume(throwing: ChronicleCollectorError.permissionDenied("Photos permission required"))
                }
            }
        }
    }
    
    private func requestSpeechRecognitionPermission() async throws {
        return try await withCheckedThrowingContinuation { continuation in
            SFSpeechRecognizer.requestAuthorization { status in
                switch status {
                case .authorized:
                    continuation.resume()
                default:
                    continuation.resume(throwing: ChronicleCollectorError.permissionDenied("Speech recognition permission required"))
                }
            }
        }
    }
    
    private func requestFileAccessPermission() async throws {
        // File access permissions are typically granted through system preferences
        // We can't programmatically request them
        let status = getFileAccessPermissionStatus()
        if status != .granted {
            throw ChronicleCollectorError.permissionDenied("File access permission required")
        }
    }
    
    private func requestSystemPolicyControlPermission() async throws {
        // System policy control permissions are typically granted through system preferences
        // We can't programmatically request them
        throw ChronicleCollectorError.permissionDenied("System policy control permission required")
    }
}

// MARK: - Permission Manager Extensions

extension PermissionManager {
    /// Get user-friendly permission explanation
    public func getPermissionExplanation(for type: PermissionType) -> String {
        switch type {
        case .accessibility:
            return "Chronicle needs Accessibility permission to monitor keyboard and mouse events. This allows Chronicle to track your productivity and create a comprehensive activity log."
        case .screenRecording:
            return "Chronicle needs Screen Recording permission to capture screenshots and monitor screen activity. This helps create a visual timeline of your work."
        case .inputMonitoring:
            return "Chronicle needs Input Monitoring permission to track keyboard and mouse activity. This data is used to understand your productivity patterns."
        case .microphone:
            return "Chronicle needs Microphone permission to detect when you're in meetings or calls. This helps categorize your time and productivity."
        case .camera:
            return "Chronicle needs Camera permission to detect camera usage during video calls and meetings."
        case .contacts:
            return "Chronicle needs Contacts permission to enrich meeting and communication data with contact information."
        case .calendars:
            return "Chronicle needs Calendar permission to correlate your activity with scheduled events and meetings."
        case .reminders:
            return "Chronicle needs Reminders permission to correlate your activity with tasks and to-do items."
        case .photos:
            return "Chronicle needs Photos permission to monitor screenshot activity and image-related work."
        case .speechRecognition:
            return "Chronicle needs Speech Recognition permission to transcribe meetings and calls for better productivity insights."
        case .fileAccess, .fullDiskAccess:
            return "Chronicle needs File Access permission to monitor file system activity and track document-related work."
        case .systemPolicyControl:
            return "Chronicle needs System Policy Control permission to monitor system-wide activity and network usage."
        }
    }
    
    /// Check if permission can be requested programmatically
    public func canRequestProgrammatically(_ type: PermissionType) -> Bool {
        switch type {
        case .microphone, .camera, .contacts, .calendars, .reminders, .photos, .speechRecognition:
            return true
        case .accessibility, .screenRecording, .inputMonitoring, .fileAccess, .fullDiskAccess, .systemPolicyControl:
            return false
        }
    }
}