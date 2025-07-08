//
//  FSMonCollector.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import CoreServices
import os.log

/// File system monitoring collector using FSEvents
public class FSMonCollector: CollectorBase {
    private var eventStream: FSEventStreamRef?
    private let permissionManager: PermissionManager
    private let monitoredPaths: [String]
    private var isMonitoring = false
    
    public init(configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter,
                permissionManager: PermissionManager = PermissionManager(),
                monitoredPaths: [String] = ["/Users", "/Applications", "/Documents"]) {
        self.permissionManager = permissionManager
        self.monitoredPaths = monitoredPaths
        
        super.init(
            identifier: "fs_mon",
            displayName: "File System Monitor",
            eventTypes: [.fileSystemChange],
            configuration: configuration,
            ringBufferWriter: ringBufferWriter
        )
    }
    
    // MARK: - CollectorProtocol Implementation
    
    public override func checkPermissions() -> Bool {
        return permissionManager.checkPermission(.fileAccess) == .granted
    }
    
    public override func requestPermissions() async throws {
        try await permissionManager.requestPermission(.fileAccess)
    }
    
    public override func startCollector() throws {
        guard checkPermissions() else {
            throw ChronicleCollectorError.permissionDenied("File access permission required")
        }
        
        try startFSEventStream()
        isMonitoring = true
        
        logger.info("File system monitor collector started successfully")
    }
    
    public override func stopCollector() throws {
        stopFSEventStream()
        isMonitoring = false
        
        logger.info("File system monitor collector stopped")
    }
    
    // MARK: - FSEvents Implementation
    
    private func startFSEventStream() throws {
        let pathsToWatch = monitoredPaths as CFArray
        let context = UnsafeMutablePointer<FSEventStreamContext>.allocate(capacity: 1)
        
        context.pointee = FSEventStreamContext(
            version: 0,
            info: Unmanaged.passUnretained(self).toOpaque(),
            retain: nil,
            release: nil,
            copyDescription: nil
        )
        
        eventStream = FSEventStreamCreate(
            kCFAllocatorDefault,
            { (streamRef, clientCallBackInfo, numEvents, eventPaths, eventFlags, eventIds) in
                guard let info = clientCallBackInfo else { return }
                let collector = Unmanaged<FSMonCollector>.fromOpaque(info).takeUnretainedValue()
                collector.handleFSEvents(streamRef: streamRef, numEvents: numEvents, eventPaths: eventPaths, eventFlags: eventFlags, eventIds: eventIds)
            },
            context,
            pathsToWatch,
            FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
            1.0, // Latency
            FSEventStreamCreateFlags(kFSEventStreamCreateFlagUseCFTypes | kFSEventStreamCreateFlagFileEvents)
        )
        
        guard let stream = eventStream else {
            context.deallocate()
            throw ChronicleCollectorError.systemError("Failed to create FSEvent stream")
        }
        
        FSEventStreamScheduleWithRunLoop(stream, CFRunLoopGetCurrent(), CFRunLoopMode.defaultMode.rawValue)
        
        if !FSEventStreamStart(stream) {
            FSEventStreamInvalidate(stream)
            FSEventStreamRelease(stream)
            eventStream = nil
            context.deallocate()
            throw ChronicleCollectorError.systemError("Failed to start FSEvent stream")
        }
        
        context.deallocate()
    }
    
    private func stopFSEventStream() {
        guard let stream = eventStream else { return }
        
        FSEventStreamStop(stream)
        FSEventStreamInvalidate(stream)
        FSEventStreamRelease(stream)
        eventStream = nil
    }
    
    private func handleFSEvents(streamRef: ConstFSEventStreamRef, numEvents: Int, eventPaths: UnsafeMutableRawPointer, eventFlags: UnsafePointer<FSEventStreamEventFlags>, eventIds: UnsafePointer<FSEventStreamEventId>) {
        guard isRunning && isMonitoring else { return }
        
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        let paths = Unmanaged<CFArray>.fromOpaque(eventPaths).takeUnretainedValue() as! [String]
        
        for i in 0..<numEvents {
            let path = paths[i]
            let flags = eventFlags[i]
            let eventId = eventIds[i]
            
            handleSingleFSEvent(path: path, flags: flags, eventId: eventId)
        }
        
        updateActivity()
    }
    
    private func handleSingleFSEvent(path: String, flags: FSEventStreamEventFlags, eventId: FSEventStreamEventId) {
        // Skip if path should be excluded
        if shouldExcludePath(path) {
            return
        }
        
        // Determine change type
        let changeType = determineChangeType(flags)
        
        // Get file information
        let isDirectory = (flags & FSEventStreamEventFlags(kFSEventStreamEventFlagItemIsDir)) != 0
        let fileSize = getFileSize(path)
        let modificationDate = getModificationDate(path)
        
        // Create event data
        let eventData = FileSystemChangeEventData(
            path: path,
            eventFlags: UInt64(flags),
            eventId: UInt64(eventId),
            isDirectory: isDirectory,
            changeType: changeType,
            fileSize: fileSize,
            modificationDate: modificationDate
        )
        
        emitFileSystemEvent(eventData)
    }
    
    private func emitFileSystemEvent(_ eventData: FileSystemChangeEventData) {
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            let chronicleEvent = createEvent(type: .fileSystemChange, data: jsonData, metadata: [
                "path": eventData.path,
                "change_type": eventData.changeType,
                "is_directory": String(eventData.isDirectory),
                "file_size": eventData.fileSize?.description ?? "unknown"
            ])
            
            emitEvent(chronicleEvent)
            
        } catch {
            logger.error("Failed to encode file system event: \(error)")
        }
    }
    
    // MARK: - Utility Methods
    
    private func determineChangeType(_ flags: FSEventStreamEventFlags) -> String {
        var changeTypes: [String] = []
        
        if (flags & FSEventStreamEventFlags(kFSEventStreamEventFlagItemCreated)) != 0 {
            changeTypes.append("created")
        }
        if (flags & FSEventStreamEventFlags(kFSEventStreamEventFlagItemRemoved)) != 0 {
            changeTypes.append("removed")
        }
        if (flags & FSEventStreamEventFlags(kFSEventStreamEventFlagItemRenamed)) != 0 {
            changeTypes.append("renamed")
        }
        if (flags & FSEventStreamEventFlags(kFSEventStreamEventFlagItemModified)) != 0 {
            changeTypes.append("modified")
        }
        if (flags & FSEventStreamEventFlags(kFSEventStreamEventFlagItemXattrMod)) != 0 {
            changeTypes.append("metadata_modified")
        }
        if (flags & FSEventStreamEventFlags(kFSEventStreamEventFlagItemChangeOwner)) != 0 {
            changeTypes.append("owner_changed")
        }
        
        return changeTypes.isEmpty ? "unknown" : changeTypes.joined(separator: ",")
    }
    
    private func getFileSize(_ path: String) -> Int64? {
        let url = URL(fileURLWithPath: path)
        
        do {
            let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
            return attributes[.size] as? Int64
        } catch {
            return nil
        }
    }
    
    private func getModificationDate(_ path: String) -> TimeInterval? {
        let url = URL(fileURLWithPath: path)
        
        do {
            let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
            return (attributes[.modificationDate] as? Date)?.timeIntervalSince1970
        } catch {
            return nil
        }
    }
    
    private func shouldExcludePath(_ path: String) -> Bool {
        let privacyConfig = ConfigManager.shared.config.privacy
        
        // Check against excluded patterns
        let excludePatterns = [
            "/.Trash/",
            "/tmp/",
            "/var/tmp/",
            "/.git/",
            "/node_modules/",
            ".DS_Store",
            ".localized"
        ]
        
        for pattern in excludePatterns {
            if path.contains(pattern) {
                return true
            }
        }
        
        // Check against privacy settings
        if privacyConfig.enableSensitiveDataFiltering {
            let sensitivePatterns = [
                "/Library/Keychains/",
                "/Library/Application Support/com.apple.TCC/",
                "/.ssh/",
                "/Documents/Private/"
            ]
            
            for pattern in sensitivePatterns {
                if path.contains(pattern) {
                    return true
                }
            }
        }
        
        return false
    }
    
    /// Get file system statistics
    public func getFileSystemStatistics() -> [String: Any] {
        return [
            "is_monitoring": isMonitoring,
            "monitored_paths": monitoredPaths,
            "event_stream_active": eventStream != nil
        ]
    }
    
    /// Get file system info for a path
    public func getFileSystemInfo(_ path: String) -> [String: Any] {
        let url = URL(fileURLWithPath: path)
        var info: [String: Any] = ["path": path]
        
        do {
            let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
            
            info["exists"] = true
            info["size"] = attributes[.size] as? Int64 ?? 0
            info["creation_date"] = (attributes[.creationDate] as? Date)?.timeIntervalSince1970
            info["modification_date"] = (attributes[.modificationDate] as? Date)?.timeIntervalSince1970
            info["is_directory"] = (attributes[.type] as? FileAttributeType) == .typeDirectory
            info["permissions"] = attributes[.posixPermissions] as? Int ?? 0
            info["owner"] = attributes[.ownerAccountName] as? String ?? "unknown"
            info["group"] = attributes[.groupOwnerAccountName] as? String ?? "unknown"
            
        } catch {
            info["exists"] = false
            info["error"] = error.localizedDescription
        }
        
        return info
    }
}