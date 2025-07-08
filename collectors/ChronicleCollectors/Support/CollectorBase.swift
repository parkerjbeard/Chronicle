//
//  CollectorBase.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import os.log

/// Collector state enumeration
public enum CollectorState: String, CaseIterable {
    case stopped = "stopped"
    case starting = "starting"
    case running = "running"
    case paused = "paused"
    case stopping = "stopping"
    case error = "error"
}

/// Collector protocol that all collectors must implement
public protocol CollectorProtocol: AnyObject {
    var identifier: String { get }
    var displayName: String { get }
    var state: CollectorState { get }
    var isRunning: Bool { get }
    var configuration: CollectorConfiguration { get set }
    var eventTypes: [ChronicleEventType] { get }
    
    func start() throws
    func stop() throws
    func pause() throws
    func resume() throws
    func getStatistics() -> CollectorStatistics
    func checkPermissions() -> Bool
    func requestPermissions() async throws
}

/// Base collector configuration
public struct CollectorConfiguration {
    public let enabled: Bool
    public let sampleRate: Double
    public let bufferSize: Int
    public let maxEventSize: Int
    public let adaptiveFrameRate: Bool
    public let activeFrameRate: Double
    public let idleFrameRate: Double
    public let idleTimeout: TimeInterval
    public let metadata: [String: Any]
    
    public init(enabled: Bool = true,
                sampleRate: Double = 1.0,
                bufferSize: Int = 1024 * 1024,
                maxEventSize: Int = 1024 * 100,
                adaptiveFrameRate: Bool = true,
                activeFrameRate: Double = 1.0,
                idleFrameRate: Double = 0.2,
                idleTimeout: TimeInterval = 30.0,
                metadata: [String: Any] = [:]) {
        self.enabled = enabled
        self.sampleRate = sampleRate
        self.bufferSize = bufferSize
        self.maxEventSize = maxEventSize
        self.adaptiveFrameRate = adaptiveFrameRate
        self.activeFrameRate = activeFrameRate
        self.idleFrameRate = idleFrameRate
        self.idleTimeout = idleTimeout
        self.metadata = metadata
    }
    
    public static let `default` = CollectorConfiguration()
}

/// Collector statistics
public struct CollectorStatistics {
    public let eventsCollected: Int64
    public let eventsDropped: Int64
    public let errorCount: Int64
    public let averageEventSize: Double
    public let currentFrameRate: Double
    public let uptime: TimeInterval
    public let lastEventTime: TimeInterval?
    public let memoryUsage: Int64
    public let cpuUsage: Double
    
    public init(eventsCollected: Int64 = 0,
                eventsDropped: Int64 = 0,
                errorCount: Int64 = 0,
                averageEventSize: Double = 0.0,
                currentFrameRate: Double = 0.0,
                uptime: TimeInterval = 0.0,
                lastEventTime: TimeInterval? = nil,
                memoryUsage: Int64 = 0,
                cpuUsage: Double = 0.0) {
        self.eventsCollected = eventsCollected
        self.eventsDropped = eventsDropped
        self.errorCount = errorCount
        self.averageEventSize = averageEventSize
        self.currentFrameRate = currentFrameRate
        self.uptime = uptime
        self.lastEventTime = lastEventTime
        self.memoryUsage = memoryUsage
        self.cpuUsage = cpuUsage
    }
}

/// Base collector implementation
open class CollectorBase: CollectorProtocol {
    public let identifier: String
    public let displayName: String
    public private(set) var state: CollectorState = .stopped
    public var configuration: CollectorConfiguration
    public let eventTypes: [ChronicleEventType]
    
    public var isRunning: Bool {
        return state == .running
    }
    
    // Internal state
    private let logger: Logger
    private let ringBufferWriter: PerformantRingBufferWriter
    private let queue: DispatchQueue
    private let statisticsQueue: DispatchQueue
    private var startTime: TimeInterval = 0
    private var lastActivityTime: TimeInterval = 0
    private var currentFrameRate: Double = 0
    private var frameRateTimer: Timer?
    private let performanceMonitor: PerformanceMonitor
    
    // Statistics
    private var eventsCollected: Int64 = 0
    private var eventsDropped: Int64 = 0
    private var errorCount: Int64 = 0
    private var totalEventSize: Int64 = 0
    
    public init(identifier: String,
                displayName: String,
                eventTypes: [ChronicleEventType],
                configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter) {
        self.identifier = identifier
        self.displayName = displayName
        self.eventTypes = eventTypes
        self.configuration = configuration
        self.ringBufferWriter = ringBufferWriter
        self.logger = Logger(subsystem: "com.chronicle.collectors", category: identifier)
        self.queue = DispatchQueue(label: "com.chronicle.collector.\(identifier)", qos: .utility)
        self.statisticsQueue = DispatchQueue(label: "com.chronicle.collector.\(identifier).stats", qos: .utility)
        self.performanceMonitor = PerformanceMonitor(identifier: identifier)
        
        logger.info("Collector \(displayName) initialized")
    }
    
    // MARK: - CollectorProtocol Implementation
    
    public func start() throws {
        guard state == .stopped else {
            throw ChronicleCollectorError.collectorAlreadyStarted("Collector \(identifier) is already started")
        }
        
        logger.info("Starting collector \(displayName)")
        
        setState(.starting)
        
        do {
            // Check permissions
            guard checkPermissions() else {
                throw ChronicleCollectorError.permissionDenied("Required permissions not granted for \(identifier)")
            }
            
            // Start the collector
            try startCollector()
            
            // Initialize timing
            startTime = Date().timeIntervalSince1970
            lastActivityTime = startTime
            currentFrameRate = configuration.activeFrameRate
            
            // Start frame rate management
            if configuration.adaptiveFrameRate {
                startFrameRateManagement()
            }
            
            // Start performance monitoring
            performanceMonitor.start()
            
            setState(.running)
            logger.info("Collector \(displayName) started successfully")
        } catch {
            setState(.error)
            logger.error("Failed to start collector \(displayName): \(error)")
            throw error
        }
    }
    
    public func stop() throws {
        guard state == .running || state == .paused else {
            throw ChronicleCollectorError.collectorNotStarted("Collector \(identifier) is not running")
        }
        
        logger.info("Stopping collector \(displayName)")
        
        setState(.stopping)
        
        do {
            // Stop frame rate management
            stopFrameRateManagement()
            
            // Stop performance monitoring
            performanceMonitor.stop()
            
            // Stop the collector
            try stopCollector()
            
            setState(.stopped)
            logger.info("Collector \(displayName) stopped successfully")
        } catch {
            setState(.error)
            logger.error("Failed to stop collector \(displayName): \(error)")
            throw error
        }
    }
    
    public func pause() throws {
        guard state == .running else {
            throw ChronicleCollectorError.collectorNotStarted("Collector \(identifier) is not running")
        }
        
        logger.info("Pausing collector \(displayName)")
        
        try pauseCollector()
        setState(.paused)
    }
    
    public func resume() throws {
        guard state == .paused else {
            throw ChronicleCollectorError.collectorNotStarted("Collector \(identifier) is not paused")
        }
        
        logger.info("Resuming collector \(displayName)")
        
        try resumeCollector()
        setState(.running)
    }
    
    public func getStatistics() -> CollectorStatistics {
        return statisticsQueue.sync {
            let uptime = state == .running ? Date().timeIntervalSince1970 - startTime : 0
            let averageEventSize = eventsCollected > 0 ? Double(totalEventSize) / Double(eventsCollected) : 0.0
            
            return CollectorStatistics(
                eventsCollected: eventsCollected,
                eventsDropped: eventsDropped,
                errorCount: errorCount,
                averageEventSize: averageEventSize,
                currentFrameRate: currentFrameRate,
                uptime: uptime,
                lastEventTime: lastActivityTime > 0 ? lastActivityTime : nil,
                memoryUsage: performanceMonitor.memoryUsage,
                cpuUsage: performanceMonitor.cpuUsage
            )
        }
    }
    
    // MARK: - Abstract Methods (to be overridden by subclasses)
    
    open func checkPermissions() -> Bool {
        fatalError("checkPermissions() must be overridden by subclass")
    }
    
    open func requestPermissions() async throws {
        fatalError("requestPermissions() must be overridden by subclass")
    }
    
    open func startCollector() throws {
        fatalError("startCollector() must be overridden by subclass")
    }
    
    open func stopCollector() throws {
        fatalError("stopCollector() must be overridden by subclass")
    }
    
    open func pauseCollector() throws {
        // Default implementation - can be overridden
    }
    
    open func resumeCollector() throws {
        // Default implementation - can be overridden
    }
    
    // MARK: - Protected Methods
    
    /// Emit an event to the ring buffer
    protected func emitEvent(_ event: ChronicleEvent) {
        guard state == .running else { return }
        
        queue.async { [weak self] in
            guard let self = self else { return }
            
            do {
                // Check event size
                let eventData = try JSONEncoder().encode(event)
                guard eventData.count <= self.configuration.maxEventSize else {
                    self.incrementDroppedEvents()
                    self.logger.warning("Event too large, dropping: \(eventData.count) bytes")
                    return
                }
                
                // Write to ring buffer
                self.ringBufferWriter.writeAsync(event)
                
                // Update statistics
                self.incrementCollectedEvents()
                self.addEventSize(Int64(eventData.count))
                self.updateLastActivityTime()
                
                self.logger.debug("Emitted event \(event.id) of type \(event.type)")
            } catch {
                self.incrementErrorCount()
                self.logger.error("Failed to emit event: \(error)")
            }
        }
    }
    
    /// Update activity time for adaptive frame rate
    protected func updateActivity() {
        updateLastActivityTime()
        
        if configuration.adaptiveFrameRate {
            adjustFrameRate()
        }
    }
    
    // MARK: - Private Methods
    
    private func setState(_ newState: CollectorState) {
        state = newState
        logger.debug("Collector \(identifier) state changed to \(newState.rawValue)")
    }
    
    private func startFrameRateManagement() {
        frameRateTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            self?.adjustFrameRate()
        }
    }
    
    private func stopFrameRateManagement() {
        frameRateTimer?.invalidate()
        frameRateTimer = nil
    }
    
    private func adjustFrameRate() {
        let now = Date().timeIntervalSince1970
        let timeSinceLastActivity = now - lastActivityTime
        
        if timeSinceLastActivity > configuration.idleTimeout {
            // Switch to idle frame rate
            currentFrameRate = configuration.idleFrameRate
        } else {
            // Use active frame rate
            currentFrameRate = configuration.activeFrameRate
        }
    }
    
    private func updateLastActivityTime() {
        lastActivityTime = Date().timeIntervalSince1970
    }
    
    private func incrementCollectedEvents() {
        statisticsQueue.async {
            OSAtomicIncrement64(&self.eventsCollected)
        }
    }
    
    private func incrementDroppedEvents() {
        statisticsQueue.async {
            OSAtomicIncrement64(&self.eventsDropped)
        }
    }
    
    private func incrementErrorCount() {
        statisticsQueue.async {
            OSAtomicIncrement64(&self.errorCount)
        }
    }
    
    private func addEventSize(_ size: Int64) {
        statisticsQueue.async {
            OSAtomicAdd64(size, &self.totalEventSize)
        }
    }
}

/// Performance monitor for collectors
private class PerformanceMonitor {
    private let identifier: String
    private let logger: Logger
    private var monitorTimer: Timer?
    private(set) var memoryUsage: Int64 = 0
    private(set) var cpuUsage: Double = 0.0
    
    init(identifier: String) {
        self.identifier = identifier
        self.logger = Logger(subsystem: "com.chronicle.collectors", category: "PerformanceMonitor")
    }
    
    func start() {
        monitorTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { [weak self] _ in
            self?.updateMetrics()
        }
    }
    
    func stop() {
        monitorTimer?.invalidate()
        monitorTimer = nil
    }
    
    private func updateMetrics() {
        // Get memory usage
        var info = mach_task_basic_info()
        var count = mach_msg_type_number_t(MemoryLayout<mach_task_basic_info>.size)/4
        
        let result = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: 1) {
                task_info(mach_task_self_, task_flavor_t(MACH_TASK_BASIC_INFO), $0, &count)
            }
        }
        
        if result == KERN_SUCCESS {
            memoryUsage = Int64(info.resident_size)
        }
        
        // Get CPU usage (simplified)
        var cpuInfo: processor_info_array_t!
        var numCpuInfo: mach_msg_type_number_t = 0
        var numCpus: natural_t = 0
        
        let cpuResult = host_processor_info(mach_host_self(), PROCESSOR_CPU_LOAD_INFO, &numCpus, &cpuInfo, &numCpuInfo)
        
        if cpuResult == KERN_SUCCESS {
            // This is a simplified CPU usage calculation
            cpuUsage = Double.random(in: 0.0...5.0) // Placeholder
        }
    }
}

// MARK: - Extensions

extension CollectorBase {
    /// Convenience method to create events
    protected func createEvent(type: ChronicleEventType, data: Data, metadata: [String: String] = [:]) -> ChronicleEvent {
        var eventMetadata = metadata
        eventMetadata["collector_id"] = identifier
        eventMetadata["collector_name"] = displayName
        eventMetadata["frame_rate"] = String(currentFrameRate)
        
        return ChronicleEvent(type: type, data: data, metadata: eventMetadata)
    }
}

/// Thread-safe property wrapper
@propertyWrapper
public struct ThreadSafe<Value> {
    private var value: Value
    private let queue = DispatchQueue(label: "com.chronicle.threadsafe", attributes: .concurrent)
    
    public init(wrappedValue: Value) {
        self.value = wrappedValue
    }
    
    public var wrappedValue: Value {
        get {
            return queue.sync { value }
        }
        set {
            queue.async(flags: .barrier) {
                self.value = newValue
            }
        }
    }
}