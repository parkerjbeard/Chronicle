//
//  ScreenTapCollector.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import CoreGraphics
import ScreenCaptureKit
import AppKit
import os.log

/// Screen capture collector using ScreenCaptureKit
@available(macOS 12.3, *)
public class ScreenTapCollector: CollectorBase {
    private var captureEngine: SCStreamConfiguration?
    private var stream: SCStream?
    private let permissionManager: PermissionManager
    private var captureTimer: Timer?
    private var lastCaptureTime: Date = Date()
    private var isCapturing: Bool = false
    private let compressionQuality: CGFloat = 0.8
    private var availableContent: SCShareableContent?
    
    // Capture configuration
    private struct CaptureConfig {
        let width: Int
        let height: Int
        let pixelFormat: OSType
        let showsCursor: Bool
        let capturesShadowsOnly: Bool
        let shouldBeOpaque: Bool
        let scalesToFit: Bool
        let ignoreGlobalClipDisplay: Bool
        let ignoreMenuBar: Bool
        let ignoreDockIcon: Bool
        
        static let `default` = CaptureConfig(
            width: 1920,
            height: 1080,
            pixelFormat: kCVPixelFormatType_32BGRA,
            showsCursor: true,
            capturesShadowsOnly: false,
            shouldBeOpaque: false,
            scalesToFit: true,
            ignoreGlobalClipDisplay: false,
            ignoreMenuBar: false,
            ignoreDockIcon: false
        )
    }
    
    private let captureConfig: CaptureConfig
    
    public init(configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter,
                permissionManager: PermissionManager = PermissionManager(),
                captureConfig: CaptureConfig = .default) {
        self.permissionManager = permissionManager
        self.captureConfig = captureConfig
        
        super.init(
            identifier: "screen_tap",
            displayName: "Screen Capture",
            eventTypes: [.screenCapture],
            configuration: configuration,
            ringBufferWriter: ringBufferWriter
        )
    }
    
    // MARK: - CollectorProtocol Implementation
    
    public override func checkPermissions() -> Bool {
        return permissionManager.checkPermission(.screenRecording) == .granted
    }
    
    public override func requestPermissions() async throws {
        try await permissionManager.requestPermission(.screenRecording)
    }
    
    public override func startCollector() throws {
        guard checkPermissions() else {
            throw ChronicleCollectorError.permissionDenied("Screen recording permission required")
        }
        
        if #available(macOS 12.3, *) {
            Task {
                await startScreenCapture()
            }
        } else {
            // Fallback for older macOS versions
            startLegacyScreenCapture()
        }
        
        logger.info("Screen tap collector started successfully")
    }
    
    public override func stopCollector() throws {
        stopScreenCapture()
        logger.info("Screen tap collector stopped")
    }
    
    // MARK: - Modern ScreenCaptureKit Implementation
    
    @available(macOS 12.3, *)
    private func startScreenCapture() async {
        do {
            // Get shareable content
            availableContent = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            
            guard let content = availableContent else {
                logger.error("Failed to get shareable content")
                return
            }
            
            // Create stream configuration
            let streamConfig = SCStreamConfiguration()
            streamConfig.width = captureConfig.width
            streamConfig.height = captureConfig.height
            streamConfig.pixelFormat = captureConfig.pixelFormat
            streamConfig.showsCursor = captureConfig.showsCursor
            streamConfig.capturesShadowsOnly = captureConfig.capturesShadowsOnly
            streamConfig.shouldBeOpaque = captureConfig.shouldBeOpaque
            streamConfig.scalesToFit = captureConfig.scalesToFit
            streamConfig.ignoreGlobalClipDisplay = captureConfig.ignoreGlobalClipDisplay
            streamConfig.ignoreMenuBar = captureConfig.ignoreMenuBar
            streamConfig.ignoreDockIcon = captureConfig.ignoreDockIcon
            
            // Set up capture filter
            let filter = SCContentFilter(display: content.displays.first!, excluding: [])
            
            // Create and start stream
            stream = SCStream(filter: filter, configuration: streamConfig, delegate: self)
            
            try await stream?.startCapture()
            
            // Start periodic capture
            startPeriodicCapture()
            
        } catch {
            logger.error("Failed to start screen capture: \(error)")
            throw ChronicleCollectorError.systemError("Failed to start screen capture: \(error)")
        }
    }
    
    private func startPeriodicCapture() {
        let interval = 1.0 / currentFrameRate
        
        captureTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            self?.captureScreen()
        }
    }
    
    private func stopScreenCapture() {
        captureTimer?.invalidate()
        captureTimer = nil
        
        if #available(macOS 12.3, *) {
            Task {
                await stream?.stopCapture()
            }
        }
        
        stream = nil
        isCapturing = false
    }
    
    private func captureScreen() {
        guard !isCapturing else { return }
        
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        isCapturing = true
        
        if #available(macOS 12.3, *) {
            captureScreenModern()
        } else {
            captureScreenLegacy()
        }
    }
    
    @available(macOS 12.3, *)
    private func captureScreenModern() {
        // Screen capture with ScreenCaptureKit is handled by the delegate
        // This method is called for timing but actual capture happens in delegate
        updateActivity()
    }
    
    // MARK: - Legacy Screen Capture Implementation
    
    private func startLegacyScreenCapture() {
        startPeriodicCapture()
    }
    
    private func captureScreenLegacy() {
        guard let display = CGMainDisplayID() as CGDirectDisplayID? else {
            isCapturing = false
            return
        }
        
        // Create screenshot
        guard let image = CGDisplayCreateImage(display) else {
            logger.error("Failed to create display image")
            isCapturing = false
            return
        }
        
        // Convert to NSImage for processing
        let nsImage = NSImage(cgImage: image, size: NSSize(width: CGFloat(image.width), height: CGFloat(image.height)))
        
        // Convert to JPEG data with compression
        guard let tiffData = nsImage.tiffRepresentation,
              let bitmapRep = NSBitmapImageRep(data: tiffData) else {
            logger.error("Failed to create bitmap representation")
            isCapturing = false
            return
        }
        
        guard let jpegData = bitmapRep.representation(using: .jpeg, properties: [.compressionFactor: compressionQuality]) else {
            logger.error("Failed to create JPEG data")
            isCapturing = false
            return
        }
        
        // Create screen capture event
        let displayBounds = CGDisplayBounds(display)
        let eventData = ScreenCaptureEventData(
            imageData: jpegData,
            format: "jpeg",
            width: Int(displayBounds.width),
            height: Int(displayBounds.height),
            scale: 1.0,
            display: "main",
            region: displayBounds,
            compressionQuality: Double(compressionQuality)
        )
        
        emitScreenCaptureEvent(eventData)
        isCapturing = false
    }
    
    // MARK: - Event Emission
    
    private func emitScreenCaptureEvent(_ eventData: ScreenCaptureEventData) {
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            let chronicleEvent = createEvent(type: .screenCapture, data: jsonData, metadata: [
                "format": eventData.format,
                "width": String(eventData.width),
                "height": String(eventData.height),
                "compression_quality": String(eventData.compressionQuality)
            ])
            
            emitEvent(chronicleEvent)
            updateActivity()
            
        } catch {
            logger.error("Failed to encode screen capture event: \(error)")
        }
    }
    
    // MARK: - Adaptive Frame Rate
    
    private var currentFrameRate: Double {
        return configuration.adaptiveFrameRate ? adaptiveFrameRate : configuration.activeFrameRate
    }
    
    private var adaptiveFrameRate: Double {
        let now = Date()
        let timeSinceLastActivity = now.timeIntervalSince(lastCaptureTime)
        
        if timeSinceLastActivity > configuration.idleTimeout {
            return configuration.idleFrameRate
        } else {
            return configuration.activeFrameRate
        }
    }
    
    private func updateCaptureTimer() {
        captureTimer?.invalidate()
        startPeriodicCapture()
    }
    
    // MARK: - Screen Analysis
    
    private func analyzeScreenContent(_ imageData: Data) -> [String: Any] {
        var analysis: [String: Any] = [:]
        
        // Basic analysis
        analysis["data_size"] = imageData.count
        analysis["timestamp"] = Date().timeIntervalSince1970
        
        // Could add more sophisticated analysis here:
        // - OCR for text detection
        // - Object detection
        // - Color analysis
        // - Motion detection
        
        return analysis
    }
    
    // MARK: - Privacy and Security
    
    private func shouldCaptureScreen() -> Bool {
        // Check privacy settings
        let privacyConfig = ConfigManager.shared.config.privacy
        
        // Check if current application should be excluded
        if let frontmostApp = NSWorkspace.shared.frontmostApplication {
            if privacyConfig.excludeApplications.contains(frontmostApp.bundleIdentifier ?? "") {
                return false
            }
        }
        
        // Check for sensitive windows
        if isPasswordFieldActive() || isSecureInputActive() {
            return false
        }
        
        return true
    }
    
    private func isPasswordFieldActive() -> Bool {
        // Check if a password field is currently active
        // This is a simplified check - in practice, you'd need more sophisticated detection
        return false
    }
    
    private func isSecureInputActive() -> Bool {
        // Check if secure input is active (like Terminal with secure input)
        return IsSecureEventInputEnabled()
    }
}

// MARK: - SCStreamDelegate

@available(macOS 12.3, *)
extension ScreenTapCollector: SCStreamDelegate {
    public func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .screen else { return }
        
        // Process the sample buffer
        processSampleBuffer(sampleBuffer)
    }
    
    public func stream(_ stream: SCStream, didStopWithError error: Error) {
        logger.error("Screen capture stream stopped with error: \(error)")
        
        // Try to restart the stream after a delay
        DispatchQueue.main.asyncAfter(deadline: .now() + 5.0) {
            Task {
                await self.startScreenCapture()
            }
        }
    }
    
    private func processSampleBuffer(_ sampleBuffer: CMSampleBuffer) {
        guard let imageBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else {
            logger.error("Failed to get image buffer from sample buffer")
            return
        }
        
        // Convert to CGImage
        let ciImage = CIImage(cvImageBuffer: imageBuffer)
        let context = CIContext()
        guard let cgImage = context.createCGImage(ciImage, from: ciImage.extent) else {
            logger.error("Failed to create CGImage from CIImage")
            return
        }
        
        // Convert to NSImage and then to JPEG data
        let nsImage = NSImage(cgImage: cgImage, size: NSSize(width: cgImage.width, height: cgImage.height))
        
        guard let tiffData = nsImage.tiffRepresentation,
              let bitmapRep = NSBitmapImageRep(data: tiffData),
              let jpegData = bitmapRep.representation(using: .jpeg, properties: [.compressionFactor: compressionQuality]) else {
            logger.error("Failed to convert image to JPEG")
            return
        }
        
        // Create screen capture event
        let eventData = ScreenCaptureEventData(
            imageData: jpegData,
            format: "jpeg",
            width: cgImage.width,
            height: cgImage.height,
            scale: 1.0,
            display: "main",
            region: CGRect(x: 0, y: 0, width: cgImage.width, height: cgImage.height),
            compressionQuality: Double(compressionQuality)
        )
        
        emitScreenCaptureEvent(eventData)
    }
}

// MARK: - Screen Capture Utilities

extension ScreenTapCollector {
    /// Get available displays
    public func getAvailableDisplays() -> [CGDirectDisplayID] {
        let maxDisplays: UInt32 = 16
        var displays = [CGDirectDisplayID](repeating: 0, count: Int(maxDisplays))
        var displayCount: UInt32 = 0
        
        let result = CGGetActiveDisplayList(maxDisplays, &displays, &displayCount)
        guard result == kCGErrorSuccess else {
            logger.error("Failed to get active display list: \(result)")
            return []
        }
        
        return Array(displays[0..<Int(displayCount)])
    }
    
    /// Get display information
    public func getDisplayInfo(_ displayID: CGDirectDisplayID) -> [String: Any] {
        var info: [String: Any] = [:]
        
        let bounds = CGDisplayBounds(displayID)
        info["bounds"] = [
            "x": bounds.origin.x,
            "y": bounds.origin.y,
            "width": bounds.size.width,
            "height": bounds.size.height
        ]
        
        info["is_main"] = CGMainDisplayID() == displayID
        info["is_online"] = CGDisplayIsOnline(displayID) != 0
        info["is_active"] = CGDisplayIsActive(displayID) != 0
        info["is_sleeping"] = CGDisplayIsAsleep(displayID) != 0
        info["is_builtin"] = CGDisplayIsBuiltin(displayID) != 0
        
        // Get display name (if available)
        if let displayName = getDisplayName(displayID) {
            info["name"] = displayName
        }
        
        return info
    }
    
    private func getDisplayName(_ displayID: CGDirectDisplayID) -> String? {
        var displayName: String?
        
        // Try to get display name using IOKit
        let options = [kIODisplayOnlyPreferredName: kCFBooleanTrue]
        if let displayNames = IODisplayCreateInfoDictionary(CGDisplayIOServicePort(displayID), IOOptionBits(kIODisplayOnlyPreferredName)).takeRetainedValue() as? [String: Any],
           let localizedNames = displayNames["DisplayProductName"] as? [String: String],
           let englishName = localizedNames["en_US"] ?? localizedNames.values.first {
            displayName = englishName
        }
        
        return displayName
    }
    
    /// Get screen capture statistics
    public func getCaptureStatistics() -> [String: Any] {
        return [
            "is_capturing": isCapturing,
            "current_frame_rate": currentFrameRate,
            "last_capture_time": lastCaptureTime.timeIntervalSince1970,
            "compression_quality": compressionQuality,
            "capture_config": [
                "width": captureConfig.width,
                "height": captureConfig.height,
                "shows_cursor": captureConfig.showsCursor,
                "scales_to_fit": captureConfig.scalesToFit
            ]
        ]
    }
}