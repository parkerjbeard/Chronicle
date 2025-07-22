//
//  ChronicleCollectors.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import os.log

/// Main Chronicle Collectors framework interface
public class ChronicleCollectors: ObservableObject {
    
    // MARK: - Shared Instance
    
    public static let shared = ChronicleCollectors()
    
    // MARK: - Properties
    
    @Published public private(set) var isRunning: Bool = false
    @Published public private(set) var collectors: [String: CollectorProtocol] = [:]
    @Published public private(set) var collectorStates: [String: CollectorState] = [:]
    
    private let logger = Logger(subsystem: "com.chronicle.collectors", category: "ChronicleCollectors")
    private let configManager = ConfigManager.shared
    private let permissionManager = PermissionManager()
    private let ringBufferWriter: PerformantRingBufferWriter
    private let queue = DispatchQueue(label: "com.chronicle.collectors.main", qos: .utility)
    
    // Collectors
    private var keyTapCollector: KeyTapCollector?
    private var screenTapCollector: ScreenTapCollector?
    private var windowMonCollector: WindowMonCollector?
    private var pointerMonCollector: PointerMonCollector?
    private var clipMonCollector: ClipMonCollector?
    private var fsMonCollector: FSMonCollector?
    private var audioMonCollector: AudioMonCollector?
    private var netMonCollector: NetMonCollector?
    private var driveMonCollector: DriveMonCollector?
    
    // MARK: - Initialization
    
    private init() {
        self.ringBufferWriter = PerformantRingBufferWriter(config: configManager.config.ringBuffer)
        
        setupCollectors()
        setupStateObservation()
        
        logger.info("Chronicle Collectors framework initialized")
    }
    
    private func setupCollectors() {
        // Initialize all collectors
        keyTapCollector = KeyTapCollector(
            configuration: configManager.getCollectorConfig("key_tap"),
            ringBufferWriter: ringBufferWriter,
            permissionManager: permissionManager
        )
        
        if #available(macOS 12.3, *) {
            screenTapCollector = ScreenTapCollector(
                configuration: configManager.getCollectorConfig("screen_tap"),
                ringBufferWriter: ringBufferWriter,
                permissionManager: permissionManager
            )
        }
        
        windowMonCollector = WindowMonCollector(
            configuration: configManager.getCollectorConfig("window_mon"),
            ringBufferWriter: ringBufferWriter,
            permissionManager: permissionManager
        )
        
        pointerMonCollector = PointerMonCollector(
            configuration: configManager.getCollectorConfig("pointer_mon"),
            ringBufferWriter: ringBufferWriter,
            permissionManager: permissionManager
        )
        
        clipMonCollector = ClipMonCollector(
            configuration: configManager.getCollectorConfig("clip_mon"),
            ringBufferWriter: ringBufferWriter,
            permissionManager: permissionManager
        )
        
        fsMonCollector = FSMonCollector(
            configuration: configManager.getCollectorConfig("fs_mon"),
            ringBufferWriter: ringBufferWriter,
            permissionManager: permissionManager
        )
        
        audioMonCollector = AudioMonCollector(
            configuration: configManager.getCollectorConfig("audio_mon"),
            ringBufferWriter: ringBufferWriter,
            permissionManager: permissionManager
        )
        
        netMonCollector = NetMonCollector(
            configuration: configManager.getCollectorConfig("net_mon"),
            ringBufferWriter: ringBufferWriter,
            permissionManager: permissionManager
        )
        
        driveMonCollector = DriveMonCollector(
            configuration: configManager.getCollectorConfig("drive_mon"),
            ringBufferWriter: ringBufferWriter,
            permissionManager: permissionManager,
            targetDrives: configManager.getAutoBackupTargetDrives()
        )
        
        // Register collectors
        if let collector = keyTapCollector {
            collectors[collector.identifier] = collector
        }
        if let collector = screenTapCollector {
            collectors[collector.identifier] = collector
        }
        if let collector = windowMonCollector {
            collectors[collector.identifier] = collector
        }
        if let collector = pointerMonCollector {
            collectors[collector.identifier] = collector
        }
        if let collector = clipMonCollector {
            collectors[collector.identifier] = collector
        }
        if let collector = fsMonCollector {
            collectors[collector.identifier] = collector
        }
        if let collector = audioMonCollector {
            collectors[collector.identifier] = collector
        }
        if let collector = netMonCollector {
            collectors[collector.identifier] = collector
        }
        if let collector = driveMonCollector {
            collectors[collector.identifier] = collector
        }
        
        updateCollectorStates()
    }
    
    private func setupStateObservation() {
        // Set up timer to periodically update collector states
        Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            self?.updateCollectorStates()
        }
    }
    
    private func updateCollectorStates() {
        for (id, collector) in collectors {
            collectorStates[id] = collector.state
        }
    }
    
    // MARK: - Public API
    
    /// Start all enabled collectors
    public func startCollectors() async throws {
        logger.info("Starting Chronicle collectors")
        
        // Check and request permissions first
        try await requestAllPermissions()
        
        var startedCollectors: [String] = []
        var failedCollectors: [String: Error] = [:]
        
        for (id, collector) in collectors {
            let config = configManager.getCollectorConfig(id)
            
            guard config.enabled else {
                logger.info("Skipping disabled collector: \(id)")
                continue
            }
            
            do {
                try collector.start()
                startedCollectors.append(id)
                logger.info("Started collector: \(id)")
            } catch {
                failedCollectors[id] = error
                logger.error("Failed to start collector \(id): \(error)")
            }
        }
        
        if !startedCollectors.isEmpty {
            isRunning = true
            logger.info("Started \(startedCollectors.count) collectors successfully")
        }
        
        if !failedCollectors.isEmpty {
            logger.warning("Failed to start \(failedCollectors.count) collectors")
            // Could throw an aggregate error here if needed
        }
        
        updateCollectorStates()
    }
    
    /// Stop all collectors
    public func stopCollectors() throws {
        logger.info("Stopping Chronicle collectors")
        
        var stoppedCollectors: [String] = []
        var failedCollectors: [String: Error] = [:]
        
        for (id, collector) in collectors {
            guard collector.isRunning else { continue }
            
            do {
                try collector.stop()
                stoppedCollectors.append(id)
                logger.info("Stopped collector: \(id)")
            } catch {
                failedCollectors[id] = error
                logger.error("Failed to stop collector \(id): \(error)")
            }
        }
        
        isRunning = false
        logger.info("Stopped \(stoppedCollectors.count) collectors")
        
        if !failedCollectors.isEmpty {
            logger.warning("Failed to stop \(failedCollectors.count) collectors")
        }
        
        updateCollectorStates()
    }
    
    /// Start specific collector
    public func startCollector(_ collectorId: String) throws {
        guard let collector = collectors[collectorId] else {
            throw ChronicleCollectorError.configurationError("Collector not found: \(collectorId)")
        }
        
        try collector.start()
        updateCollectorStates()
        
        logger.info("Started collector: \(collectorId)")
    }
    
    /// Stop specific collector
    public func stopCollector(_ collectorId: String) throws {
        guard let collector = collectors[collectorId] else {
            throw ChronicleCollectorError.configurationError("Collector not found: \(collectorId)")
        }
        
        try collector.stop()
        updateCollectorStates()
        
        logger.info("Stopped collector: \(collectorId)")
    }
    
    /// Pause specific collector
    public func pauseCollector(_ collectorId: String) throws {
        guard let collector = collectors[collectorId] else {
            throw ChronicleCollectorError.configurationError("Collector not found: \(collectorId)")
        }
        
        try collector.pause()
        updateCollectorStates()
        
        logger.info("Paused collector: \(collectorId)")
    }
    
    /// Resume specific collector
    public func resumeCollector(_ collectorId: String) throws {
        guard let collector = collectors[collectorId] else {
            throw ChronicleCollectorError.configurationError("Collector not found: \(collectorId)")
        }
        
        try collector.resume()
        updateCollectorStates()
        
        logger.info("Resumed collector: \(collectorId)")
    }
    
    // MARK: - Permissions
    
    /// Check all required permissions
    public func checkAllPermissions() -> [PermissionType: PermissionStatus] {
        let requiredPermissions = permissionManager.getRequiredPermissions(for: Array(collectors.keys))
        var permissionStatus: [PermissionType: PermissionStatus] = [:]
        
        for permission in requiredPermissions {
            permissionStatus[permission] = permissionManager.checkPermission(permission)
        }
        
        return permissionStatus
    }
    
    /// Request all required permissions
    public func requestAllPermissions() async throws {
        let requiredPermissions = permissionManager.getRequiredPermissions(for: Array(collectors.keys))
        
        for permission in requiredPermissions {
            if permissionManager.checkPermission(permission) != .granted {
                try await permissionManager.requestPermission(permission)
            }
        }
    }
    
    /// Check if all required permissions are granted
    public func hasAllRequiredPermissions() -> Bool {
        let requiredPermissions = permissionManager.getRequiredPermissions(for: Array(collectors.keys))
        return permissionManager.hasAllRequiredPermissions(for: requiredPermissions)
    }
    
    // MARK: - Statistics and Monitoring
    
    /// Get statistics for all collectors
    public func getAllStatistics() -> [String: CollectorStatistics] {
        var stats: [String: CollectorStatistics] = [:]
        
        for (id, collector) in collectors {
            stats[id] = collector.getStatistics()
        }
        
        return stats
    }
    
    /// Get statistics for specific collector
    public func getCollectorStatistics(_ collectorId: String) -> CollectorStatistics? {
        return collectors[collectorId]?.getStatistics()
    }
    
    /// Get ring buffer statistics
    public func getRingBufferStatistics() -> RingBufferStatistics {
        return ringBufferWriter.getStatistics()
    }
    
    /// Get overall system health
    public func getSystemHealth() -> SystemHealth {
        let stats = getAllStatistics()
        let ringBufferStats = getRingBufferStatistics()
        
        let totalEventsCollected = stats.values.reduce(0) { $0 + $1.eventsCollected }
        let totalEventsDropped = stats.values.reduce(0) { $0 + $1.eventsDropped }
        let totalErrors = stats.values.reduce(0) { $0 + $1.errorCount }
        
        let runningCollectors = collectors.values.filter { $0.isRunning }.count
        let totalCollectors = collectors.count
        
        let averageCpuUsage = stats.values.map { $0.cpuUsage }.reduce(0, +) / Double(stats.count)
        let totalMemoryUsage = stats.values.reduce(0) { $0 + $1.memoryUsage }
        
        return SystemHealth(
            isHealthy: totalErrors < 10 && ringBufferStats.utilizationPercentage < 80,
            runningCollectors: runningCollectors,
            totalCollectors: totalCollectors,
            totalEventsCollected: totalEventsCollected,
            totalEventsDropped: totalEventsDropped,
            totalErrors: totalErrors,
            ringBufferUtilization: ringBufferStats.utilizationPercentage,
            averageCpuUsage: averageCpuUsage,
            totalMemoryUsage: totalMemoryUsage,
            lastUpdateTime: Date()
        )
    }
    
    // MARK: - Configuration
    
    /// Update collector configuration
    public func updateCollectorConfiguration(_ collectorId: String, config: CollectorConfiguration) throws {
        guard collectors[collectorId] != nil else {
            throw ChronicleCollectorError.configurationError("Collector not found: \(collectorId)")
        }
        
        try configManager.updateCollectorConfig(collectorId, config: config)
        
        // Update the collector's configuration
        collectors[collectorId]?.configuration = config
        
        logger.info("Updated configuration for collector: \(collectorId)")
    }
    
    /// Reload configuration
    public func reloadConfiguration() {
        configManager.reloadConfiguration()
        
        // Update all collector configurations
        for (id, collector) in collectors {
            collector.configuration = configManager.getCollectorConfig(id)
        }
        
        logger.info("Reloaded configuration for all collectors")
    }
    
    // MARK: - Data Management
    
    /// Clear ring buffer
    public func clearRingBuffer() {
        ringBufferWriter.clear()
        logger.info("Cleared ring buffer")
    }
    
    /// Flush ring buffer
    public func flushRingBuffer() {
        ringBufferWriter.flush()
        logger.info("Flushed ring buffer")
    }
    
    // MARK: - Utility Methods
    
    /// Get available collector types
    public func getAvailableCollectorTypes() -> [String: String] {
        var types: [String: String] = [:]
        
        for (id, collector) in collectors {
            types[id] = collector.displayName
        }
        
        return types
    }
    
    /// Get collector information
    public func getCollectorInfo(_ collectorId: String) -> CollectorInfo? {
        guard let collector = collectors[collectorId] else { return nil }
        
        return CollectorInfo(
            identifier: collector.identifier,
            displayName: collector.displayName,
            eventTypes: collector.eventTypes,
            state: collector.state,
            isRunning: collector.isRunning,
            configuration: collector.configuration,
            statistics: collector.getStatistics()
        )
    }
}

// MARK: - Supporting Types

/// System health information
public struct SystemHealth {
    public let isHealthy: Bool
    public let runningCollectors: Int
    public let totalCollectors: Int
    public let totalEventsCollected: Int64
    public let totalEventsDropped: Int64
    public let totalErrors: Int64
    public let ringBufferUtilization: Double
    public let averageCpuUsage: Double
    public let totalMemoryUsage: Int64
    public let lastUpdateTime: Date
}

/// Collector information
public struct CollectorInfo {
    public let identifier: String
    public let displayName: String
    public let eventTypes: [ChronicleEventType]
    public let state: CollectorState
    public let isRunning: Bool
    public let configuration: CollectorConfiguration
    public let statistics: CollectorStatistics
}

// MARK: - Extensions

extension ChronicleCollectors {
    /// Convenience method to start collectors with specific IDs
    public func startCollectors(_ collectorIds: [String]) async throws {
        for id in collectorIds {
            try startCollector(id)
        }
    }
    
    /// Convenience method to stop collectors with specific IDs
    public func stopCollectors(_ collectorIds: [String]) throws {
        for id in collectorIds {
            try stopCollector(id)
        }
    }
    
    /// Check if any collectors are running
    public var hasRunningCollectors: Bool {
        return collectors.values.contains { $0.isRunning }
    }
    
    /// Get running collectors
    public var runningCollectors: [String] {
        return collectors.compactMap { (id, collector) in
            collector.isRunning ? id : nil
        }
    }
    
    /// Get stopped collectors
    public var stoppedCollectors: [String] {
        return collectors.compactMap { (id, collector) in
            !collector.isRunning ? id : nil
        }
    }
}