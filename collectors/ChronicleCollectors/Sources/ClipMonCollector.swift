//
//  ClipMonCollector.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import AppKit
import CryptoKit
import os.log

/// Clipboard monitoring collector
public class ClipMonCollector: CollectorBase {
    private let pasteboard = NSPasteboard.general
    private var monitoringTimer: Timer?
    private var lastChangeCount: Int = 0
    private var lastClipboardHash: String = ""
    private let permissionManager: PermissionManager
    
    public init(configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter,
                permissionManager: PermissionManager = PermissionManager()) {
        self.permissionManager = permissionManager
        
        super.init(
            identifier: "clip_mon",
            displayName: "Clipboard Monitor",
            eventTypes: [.clipboardChange],
            configuration: configuration,
            ringBufferWriter: ringBufferWriter
        )
    }
    
    // MARK: - CollectorProtocol Implementation
    
    public override func checkPermissions() -> Bool {
        return permissionManager.checkPermission(.accessibility) == .granted
    }
    
    public override func requestPermissions() async throws {
        try await permissionManager.requestPermission(.accessibility)
    }
    
    public override func startCollector() throws {
        guard checkPermissions() else {
            throw ChronicleCollectorError.permissionDenied("Accessibility permission required")
        }
        
        // Initialize with current state
        lastChangeCount = pasteboard.changeCount
        lastClipboardHash = getCurrentClipboardHash()
        
        // Start monitoring
        startMonitoring()
        
        logger.info("Clipboard monitor collector started successfully")
    }
    
    public override func stopCollector() throws {
        stopMonitoring()
        logger.info("Clipboard monitor collector stopped")
    }
    
    // MARK: - Monitoring
    
    private func startMonitoring() {
        let interval = 1.0 / currentFrameRate
        
        monitoringTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            self?.checkClipboardChanges()
        }
    }
    
    private func stopMonitoring() {
        monitoringTimer?.invalidate()
        monitoringTimer = nil
    }
    
    private func checkClipboardChanges() {
        guard isRunning else { return }
        
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        let currentChangeCount = pasteboard.changeCount
        
        // Check if clipboard has changed
        if currentChangeCount != lastChangeCount {
            handleClipboardChange(changeCount: currentChangeCount)
            lastChangeCount = currentChangeCount
            updateActivity()
        }
    }
    
    private func handleClipboardChange(changeCount: Int) {
        // Analyze clipboard content
        let types = pasteboard.types?.map { $0.rawValue } ?? []
        let hasString = pasteboard.string(forType: .string) != nil
        let hasImage = pasteboard.data(forType: .png) != nil || pasteboard.data(forType: .tiff) != nil
        let hasFiles = pasteboard.propertyList(forType: .fileURL) != nil
        
        // Calculate data size and hash
        var dataSize = 0
        var contentHash = ""
        
        if let stringData = pasteboard.string(forType: .string)?.data(using: .utf8) {
            dataSize += stringData.count
            contentHash = calculateHash(data: stringData)
        }
        
        if let imageData = pasteboard.data(forType: .png) ?? pasteboard.data(forType: .tiff) {
            dataSize += imageData.count
            if contentHash.isEmpty {
                contentHash = calculateHash(data: imageData)
            }
        }
        
        // Skip if content hasn't actually changed (just change count incremented)
        if contentHash == lastClipboardHash && !contentHash.isEmpty {
            return
        }
        
        lastClipboardHash = contentHash
        
        // Create clipboard event data
        let eventData = ClipboardChangeEventData(
            changeCount: changeCount,
            types: types,
            hasString: hasString,
            hasImage: hasImage,
            hasFiles: hasFiles,
            dataSize: dataSize,
            hash: contentHash
        )
        
        emitClipboardEvent(eventData)
    }
    
    private func emitClipboardEvent(_ eventData: ClipboardChangeEventData) {
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            let chronicleEvent = createEvent(type: .clipboardChange, data: jsonData, metadata: [
                "data_size": String(eventData.dataSize),
                "types": eventData.types.joined(separator: ","),
                "has_string": String(eventData.hasString),
                "has_image": String(eventData.hasImage),
                "has_files": String(eventData.hasFiles)
            ])
            
            emitEvent(chronicleEvent)
            
        } catch {
            logger.error("Failed to encode clipboard event: \(error)")
        }
    }
    
    // MARK: - Utility Methods
    
    private func getCurrentClipboardHash() -> String {
        var allData = Data()
        
        if let stringData = pasteboard.string(forType: .string)?.data(using: .utf8) {
            allData.append(stringData)
        }
        
        if let imageData = pasteboard.data(forType: .png) ?? pasteboard.data(forType: .tiff) {
            allData.append(imageData)
        }
        
        return calculateHash(data: allData)
    }
    
    private func calculateHash(data: Data) -> String {
        let digest = SHA256.hash(data: data)
        return digest.compactMap { String(format: "%02x", $0) }.joined()
    }
    
    private var currentFrameRate: Double {
        return configuration.adaptiveFrameRate ? adaptiveFrameRate : configuration.activeFrameRate
    }
    
    private var adaptiveFrameRate: Double {
        // Clipboard monitoring can be less frequent when idle
        return configuration.idleFrameRate
    }
    
    // MARK: - Privacy and Security
    
    private func shouldMonitorClipboard() -> Bool {
        let privacyConfig = ConfigManager.shared.config.privacy
        
        if privacyConfig.enableSensitiveDataFiltering {
            // Check if current application should be excluded
            if let frontmostApp = NSWorkspace.shared.frontmostApplication {
                if privacyConfig.excludeApplications.contains(frontmostApp.bundleIdentifier ?? "") {
                    return false
                }
            }
        }
        
        return true
    }
    
    /// Get clipboard statistics
    public func getClipboardStatistics() -> [String: Any] {
        let currentTypes = pasteboard.types?.map { $0.rawValue } ?? []
        
        return [
            "current_change_count": pasteboard.changeCount,
            "last_monitored_change": lastChangeCount,
            "current_types": currentTypes,
            "has_content": !currentTypes.isEmpty,
            "current_hash": getCurrentClipboardHash()
        ]
    }
}