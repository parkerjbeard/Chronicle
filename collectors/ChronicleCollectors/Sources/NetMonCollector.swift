//
//  NetMonCollector.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import Network
import SystemConfiguration
import AppKit
import os.log
import Darwin // For network interface functions (getifaddrs, etc.)

/// Network activity monitoring collector
public class NetMonCollector: CollectorBase {
    private let permissionManager: PermissionManager
    private var monitoringTimer: Timer?
    private var networkMonitor: NWPathMonitor?
    private var isMonitoring = false
    private var lastNetworkStats: NetworkStats?
    
    private struct NetworkStats {
        let bytesIn: UInt64
        let bytesOut: UInt64
        let packetsIn: UInt64
        let packetsOut: UInt64
        let timestamp: Date
    }
    
    public init(configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter,
                permissionManager: PermissionManager = PermissionManager()) {
        self.permissionManager = permissionManager
        
        super.init(
            identifier: "net_mon",
            displayName: "Network Monitor",
            eventTypes: [.networkActivity],
            configuration: configuration,
            ringBufferWriter: ringBufferWriter
        )
    }
    
    // MARK: - CollectorProtocol Implementation
    
    public override func checkPermissions() -> Bool {
        // Network monitoring doesn't require specific TCC permissions
        // but might need system policy control for deep inspection
        return true
    }
    
    public override func requestPermissions() async throws {
        // No specific permissions needed for basic network monitoring
    }
    
    public override func startCollector() throws {
        startNetworkMonitoring()
        startPeriodicCollection()
        isMonitoring = true
        
        logger.info("Network monitor collector started successfully")
    }
    
    public override func stopCollector() throws {
        stopNetworkMonitoring()
        stopPeriodicCollection()
        isMonitoring = false
        
        logger.info("Network monitor collector stopped")
    }
    
    // MARK: - Network Monitoring
    
    private func startNetworkMonitoring() {
        networkMonitor = NWPathMonitor()
        
        networkMonitor?.pathUpdateHandler = { [weak self] path in
            self?.handleNetworkPathUpdate(path)
        }
        
        let queue = DispatchQueue(label: "NetworkMonitor")
        networkMonitor?.start(queue: queue)
    }
    
    private func stopNetworkMonitoring() {
        networkMonitor?.cancel()
        networkMonitor = nil
    }
    
    private func startPeriodicCollection() {
        let interval = 1.0 / currentFrameRate
        
        monitoringTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            self?.collectNetworkActivity()
        }
    }
    
    private func stopPeriodicCollection() {
        monitoringTimer?.invalidate()
        monitoringTimer = nil
    }
    
    private func handleNetworkPathUpdate(_ path: NWPath) {
        logger.debug("Network path updated: \(path.status)")
        
        // Could emit network state change events here
        if path.status == .satisfied {
            // Network is available
        } else {
            // Network is not available
        }
    }
    
    private func collectNetworkActivity() {
        guard isRunning && isMonitoring else { return }
        
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        // Get current network statistics
        let currentStats = getCurrentNetworkStats()
        
        // Calculate deltas if we have previous stats
        if let previousStats = lastNetworkStats {
            let deltaTime = currentStats.timestamp.timeIntervalSince(previousStats.timestamp)
            
            if deltaTime > 0 {
                let bytesInDelta = currentStats.bytesIn - previousStats.bytesIn
                let bytesOutDelta = currentStats.bytesOut - previousStats.bytesOut
                let packetsInDelta = currentStats.packetsIn - previousStats.packetsIn
                let packetsOutDelta = currentStats.packetsOut - previousStats.packetsOut
                
                let bandwidth = Double(bytesInDelta + bytesOutDelta) / deltaTime // bytes per second
                
                // Get active connections
                let connections = getActiveConnections()
                
                // Create network activity event
                let eventData = NetworkActivityEventData(
                    bytesIn: bytesInDelta,
                    bytesOut: bytesOutDelta,
                    packetsIn: packetsInDelta,
                    packetsOut: packetsOutDelta,
                    connectionCount: connections.count,
                    activeConnections: connections,
                    bandwidth: bandwidth
                )
                
                emitNetworkEvent(eventData)
            }
        }
        
        lastNetworkStats = currentStats
        updateActivity()
    }
    
    private func emitNetworkEvent(_ eventData: NetworkActivityEventData) {
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            let chronicleEvent = createEvent(type: .networkActivity, data: jsonData, metadata: [
                "bytes_in": String(eventData.bytesIn),
                "bytes_out": String(eventData.bytesOut),
                "connection_count": String(eventData.connectionCount),
                "bandwidth": String(format: "%.2f", eventData.bandwidth)
            ])
            
            emitEvent(chronicleEvent)
            
        } catch {
            logger.error("Failed to encode network event: \(error)")
        }
    }
    
    // MARK: - Network Statistics
    
    private func getCurrentNetworkStats() -> NetworkStats {
        // FIXED: Now uses real system network statistics instead of random data
        var bytesIn: UInt64 = 0
        var bytesOut: UInt64 = 0
        var packetsIn: UInt64 = 0
        var packetsOut: UInt64 = 0
        
        // Get network interface statistics from system
        var ifaddrs: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifaddrs) == 0 else {
            logger.error("Failed to get network interface information")
            return NetworkStats(bytesIn: 0, bytesOut: 0, packetsIn: 0, packetsOut: 0, timestamp: Date())
        }
        
        defer {
            freeifaddrs(ifaddrs)
        }
        
        var current = ifaddrs
        while let interface = current {
            let name = String(cString: interface.pointee.ifa_name)
            
            // Skip loopback and inactive interfaces
            if name.hasPrefix("lo") || (interface.pointee.ifa_flags & UInt32(IFF_UP)) == 0 {
                current = interface.pointee.ifa_next
                continue
            }
            
            // Get interface data
            if let data = interface.pointee.ifa_data {
                let networkData = data.assumingMemoryBound(to: if_data.self)
                bytesIn += UInt64(networkData.pointee.ifi_ibytes)
                bytesOut += UInt64(networkData.pointee.ifi_obytes)
                packetsIn += UInt64(networkData.pointee.ifi_ipackets)
                packetsOut += UInt64(networkData.pointee.ifi_opackets)
            }
            
            current = interface.pointee.ifa_next
        }
        
        return NetworkStats(
            bytesIn: bytesIn,
            bytesOut: bytesOut,
            packetsIn: packetsIn,
            packetsOut: packetsOut,
            timestamp: Date()
        )
    }
    
    private func getActiveConnections() -> [NetworkConnection] {
        // Simplified implementation - would use `netstat` or system calls
        // to get actual network connections
        
        let runningApps = NSWorkspace.shared.runningApplications
        let networkApps = runningApps.prefix(5) // Limit for demo
        
        return networkApps.compactMap { app in
            guard let bundleId = app.bundleIdentifier else { return nil }
            
            return NetworkConnection(
                processId: app.processIdentifier,
                processName: app.localizedName ?? "Unknown",
                localAddress: "127.0.0.1",
                localPort: UInt16.random(in: 1024...65535),
                remoteAddress: "192.168.1.1",
                remotePort: 80,
                protocol: "TCP",
                state: "ESTABLISHED",
                bytesIn: UInt64.random(in: 0...1000),
                bytesOut: UInt64.random(in: 0...1000)
            )
        }
    }
    
    // MARK: - Network Interface Information
    
    private func getNetworkInterfaces() -> [[String: Any]] {
        var interfaces: [[String: Any]] = []
        
        // Get network interface information using SystemConfiguration
        guard let interfaceNames = SCNetworkInterfaceCopyAll() as? [SCNetworkInterface] else {
            return interfaces
        }
        
        for interface in interfaceNames {
            var info: [String: Any] = [:]
            
            if let name = SCNetworkInterfaceGetLocalizedDisplayName(interface) {
                info["display_name"] = name as String
            }
            
            if let bsdName = SCNetworkInterfaceGetBSDName(interface) {
                info["bsd_name"] = bsdName as String
            }
            
            let interfaceType = SCNetworkInterfaceGetInterfaceType(interface)
            info["type"] = interfaceType as String
            
            interfaces.append(info)
        }
        
        return interfaces
    }
    
    // MARK: - Utility Methods
    
    private var currentFrameRate: Double {
        return configuration.adaptiveFrameRate ? adaptiveFrameRate : configuration.activeFrameRate
    }
    
    private var adaptiveFrameRate: Double {
        // Network monitoring can be less frequent when no activity
        return configuration.idleFrameRate
    }
    
    /// Get network system information
    public func getNetworkSystemInfo() -> [String: Any] {
        let interfaces = getNetworkInterfaces()
        let connections = getActiveConnections()
        
        var info: [String: Any] = [
            "is_monitoring": isMonitoring,
            "interfaces": interfaces,
            "active_connections": connections.count,
            "last_stats_time": lastNetworkStats?.timestamp.timeIntervalSince1970 ?? 0
        ]
        
        // Add network path information
        if let monitor = networkMonitor {
            let currentPath = monitor.currentPath
            info["network_status"] = currentPath.status == .satisfied ? "connected" : "disconnected"
            info["is_expensive"] = currentPath.isExpensive
            info["is_constrained"] = currentPath.isConstrained
            
            // Available interface types
            var availableTypes: [String] = []
            if currentPath.usesInterfaceType(.wifi) {
                availableTypes.append("wifi")
            }
            if currentPath.usesInterfaceType(.cellular) {
                availableTypes.append("cellular")
            }
            if currentPath.usesInterfaceType(.wiredEthernet) {
                availableTypes.append("ethernet")
            }
            info["interface_types"] = availableTypes
        }
        
        return info
    }
    
    /// Get bandwidth statistics
    public func getBandwidthStatistics() -> [String: Any] {
        guard let stats = lastNetworkStats else {
            return ["error": "No statistics available"]
        }
        
        return [
            "total_bytes_in": stats.bytesIn,
            "total_bytes_out": stats.bytesOut,
            "total_packets_in": stats.packetsIn,
            "total_packets_out": stats.packetsOut,
            "last_update": stats.timestamp.timeIntervalSince1970
        ]
    }
}