//
//  EventTypes.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import CoreGraphics
import AppKit

/// Event types that can be collected by Chronicle
public enum ChronicleEventType: String, CaseIterable, Codable {
    case keyTap = "key_tap"
    case screenCapture = "screen_capture"
    case windowFocus = "window_focus"
    case pointerMove = "pointer_move"
    case pointerClick = "pointer_click"
    case clipboardChange = "clipboard_change"
    case fileSystemChange = "file_system_change"
    case audioActivity = "audio_activity"
    case networkActivity = "network_activity"
    case systemActivity = "system_activity"
}

/// Base event structure for all Chronicle events
public struct ChronicleEvent: Codable {
    public let id: UUID
    public let type: ChronicleEventType
    public let timestamp: TimeInterval
    public let data: Data
    public let metadata: [String: String]
    
    public init(type: ChronicleEventType, data: Data, metadata: [String: String] = [:]) {
        self.id = UUID()
        self.type = type
        self.timestamp = Date().timeIntervalSince1970
        self.data = data
        self.metadata = metadata
    }
}

/// Key tap event data
public struct KeyTapEventData: Codable {
    public let keyCode: UInt16
    public let modifierFlags: UInt64
    public let isKeyDown: Bool
    public let characters: String?
    public let charactersIgnoringModifiers: String?
    public let location: CGPoint
    public let windowInfo: WindowInfo?
    
    public init(keyCode: UInt16, modifierFlags: UInt64, isKeyDown: Bool, 
                characters: String? = nil, charactersIgnoringModifiers: String? = nil,
                location: CGPoint = .zero, windowInfo: WindowInfo? = nil) {
        self.keyCode = keyCode
        self.modifierFlags = modifierFlags
        self.isKeyDown = isKeyDown
        self.characters = characters
        self.charactersIgnoringModifiers = charactersIgnoringModifiers
        self.location = location
        self.windowInfo = windowInfo
    }
}

/// Screen capture event data
public struct ScreenCaptureEventData: Codable {
    public let imageData: Data
    public let format: String
    public let width: Int
    public let height: Int
    public let scale: Double
    public let display: String
    public let region: CGRect
    public let compressionQuality: Double
    
    public init(imageData: Data, format: String, width: Int, height: Int, 
                scale: Double, display: String, region: CGRect, compressionQuality: Double) {
        self.imageData = imageData
        self.format = format
        self.width = width
        self.height = height
        self.scale = scale
        self.display = display
        self.region = region
        self.compressionQuality = compressionQuality
    }
}

/// Window information
public struct WindowInfo: Codable {
    public let windowId: CGWindowID
    public let processId: pid_t
    public let processName: String
    public let windowTitle: String
    public let bundleIdentifier: String?
    public let bounds: CGRect
    public let isOnScreen: Bool
    public let isActive: Bool
    public let level: Int
    public let alpha: Double
    
    public init(windowId: CGWindowID, processId: pid_t, processName: String, 
                windowTitle: String, bundleIdentifier: String? = nil, bounds: CGRect, 
                isOnScreen: Bool, isActive: Bool, level: Int, alpha: Double) {
        self.windowId = windowId
        self.processId = processId
        self.processName = processName
        self.windowTitle = windowTitle
        self.bundleIdentifier = bundleIdentifier
        self.bounds = bounds
        self.isOnScreen = isOnScreen
        self.isActive = isActive
        self.level = level
        self.alpha = alpha
    }
}

/// Window focus event data
public struct WindowFocusEventData: Codable {
    public let windowInfo: WindowInfo
    public let previousWindowInfo: WindowInfo?
    public let focusChangeReason: String
    
    public init(windowInfo: WindowInfo, previousWindowInfo: WindowInfo? = nil, 
                focusChangeReason: String) {
        self.windowInfo = windowInfo
        self.previousWindowInfo = previousWindowInfo
        self.focusChangeReason = focusChangeReason
    }
}

/// Pointer movement event data
public struct PointerMoveEventData: Codable {
    public let location: CGPoint
    public let previousLocation: CGPoint
    public let deltaX: Double
    public let deltaY: Double
    public let velocity: Double
    public let windowInfo: WindowInfo?
    
    public init(location: CGPoint, previousLocation: CGPoint, 
                deltaX: Double, deltaY: Double, velocity: Double, 
                windowInfo: WindowInfo? = nil) {
        self.location = location
        self.previousLocation = previousLocation
        self.deltaX = deltaX
        self.deltaY = deltaY
        self.velocity = velocity
        self.windowInfo = windowInfo
    }
}

/// Pointer click event data
public struct PointerClickEventData: Codable {
    public let location: CGPoint
    public let buttonNumber: Int
    public let clickCount: Int
    public let isButtonDown: Bool
    public let modifierFlags: UInt64
    public let windowInfo: WindowInfo?
    
    public init(location: CGPoint, buttonNumber: Int, clickCount: Int, 
                isButtonDown: Bool, modifierFlags: UInt64, windowInfo: WindowInfo? = nil) {
        self.location = location
        self.buttonNumber = buttonNumber
        self.clickCount = clickCount
        self.isButtonDown = isButtonDown
        self.modifierFlags = modifierFlags
        self.windowInfo = windowInfo
    }
}

/// Clipboard change event data
public struct ClipboardChangeEventData: Codable {
    public let changeCount: Int
    public let types: [String]
    public let hasString: Bool
    public let hasImage: Bool
    public let hasFiles: Bool
    public let dataSize: Int
    public let hash: String
    
    public init(changeCount: Int, types: [String], hasString: Bool, hasImage: Bool, 
                hasFiles: Bool, dataSize: Int, hash: String) {
        self.changeCount = changeCount
        self.types = types
        self.hasString = hasString
        self.hasImage = hasImage
        self.hasFiles = hasFiles
        self.dataSize = dataSize
        self.hash = hash
    }
}

/// File system change event data
public struct FileSystemChangeEventData: Codable {
    public let path: String
    public let eventFlags: UInt64
    public let eventId: UInt64
    public let isDirectory: Bool
    public let changeType: String
    public let fileSize: Int64?
    public let modificationDate: TimeInterval?
    
    public init(path: String, eventFlags: UInt64, eventId: UInt64, 
                isDirectory: Bool, changeType: String, fileSize: Int64? = nil, 
                modificationDate: TimeInterval? = nil) {
        self.path = path
        self.eventFlags = eventFlags
        self.eventId = eventId
        self.isDirectory = isDirectory
        self.changeType = changeType
        self.fileSize = fileSize
        self.modificationDate = modificationDate
    }
}

/// Audio activity event data
public struct AudioActivityEventData: Codable {
    public let isInputActive: Bool
    public let isOutputActive: Bool
    public let inputLevel: Double
    public let outputLevel: Double
    public let isMicrophoneMuted: Bool
    public let isSystemMuted: Bool
    public let activeApplications: [String]
    public let meetingDetected: Bool
    
    public init(isInputActive: Bool, isOutputActive: Bool, inputLevel: Double, 
                outputLevel: Double, isMicrophoneMuted: Bool, isSystemMuted: Bool, 
                activeApplications: [String], meetingDetected: Bool) {
        self.isInputActive = isInputActive
        self.isOutputActive = isOutputActive
        self.inputLevel = inputLevel
        self.outputLevel = outputLevel
        self.isMicrophoneMuted = isMicrophoneMuted
        self.isSystemMuted = isSystemMuted
        self.activeApplications = activeApplications
        self.meetingDetected = meetingDetected
    }
}

/// Network activity event data
public struct NetworkActivityEventData: Codable {
    public let bytesIn: UInt64
    public let bytesOut: UInt64
    public let packetsIn: UInt64
    public let packetsOut: UInt64
    public let connectionCount: Int
    public let activeConnections: [NetworkConnection]
    public let bandwidth: Double
    
    public init(bytesIn: UInt64, bytesOut: UInt64, packetsIn: UInt64, packetsOut: UInt64, 
                connectionCount: Int, activeConnections: [NetworkConnection], bandwidth: Double) {
        self.bytesIn = bytesIn
        self.bytesOut = bytesOut
        self.packetsIn = packetsIn
        self.packetsOut = packetsOut
        self.connectionCount = connectionCount
        self.activeConnections = activeConnections
        self.bandwidth = bandwidth
    }
}

/// Network connection information
public struct NetworkConnection: Codable {
    public let processId: pid_t
    public let processName: String
    public let localAddress: String
    public let localPort: UInt16
    public let remoteAddress: String
    public let remotePort: UInt16
    public let protocol: String
    public let state: String
    public let bytesIn: UInt64
    public let bytesOut: UInt64
    
    public init(processId: pid_t, processName: String, localAddress: String, 
                localPort: UInt16, remoteAddress: String, remotePort: UInt16, 
                protocol: String, state: String, bytesIn: UInt64, bytesOut: UInt64) {
        self.processId = processId
        self.processName = processName
        self.localAddress = localAddress
        self.localPort = localPort
        self.remoteAddress = remoteAddress
        self.remotePort = remotePort
        self.protocol = `protocol`
        self.state = state
        self.bytesIn = bytesIn
        self.bytesOut = bytesOut
    }
}

/// System activity event data
public struct SystemActivityEventData: Codable {
    public let cpuUsage: Double
    public let memoryUsage: Double
    public let diskUsage: Double
    public let networkUsage: Double
    public let batteryLevel: Double?
    public let isCharging: Bool?
    public let thermalState: String
    public let activeProcesses: [ProcessInfo]
    
    public init(cpuUsage: Double, memoryUsage: Double, diskUsage: Double, 
                networkUsage: Double, batteryLevel: Double? = nil, isCharging: Bool? = nil, 
                thermalState: String, activeProcesses: [ProcessInfo]) {
        self.cpuUsage = cpuUsage
        self.memoryUsage = memoryUsage
        self.diskUsage = diskUsage
        self.networkUsage = networkUsage
        self.batteryLevel = batteryLevel
        self.isCharging = isCharging
        self.thermalState = thermalState
        self.activeProcesses = activeProcesses
    }
}

/// Process information
public struct ProcessInfo: Codable {
    public let processId: pid_t
    public let processName: String
    public let bundleIdentifier: String?
    public let cpuUsage: Double
    public let memoryUsage: UInt64
    public let isActive: Bool
    public let launchDate: TimeInterval
    
    public init(processId: pid_t, processName: String, bundleIdentifier: String? = nil, 
                cpuUsage: Double, memoryUsage: UInt64, isActive: Bool, launchDate: TimeInterval) {
        self.processId = processId
        self.processName = processName
        self.bundleIdentifier = bundleIdentifier
        self.cpuUsage = cpuUsage
        self.memoryUsage = memoryUsage
        self.isActive = isActive
        self.launchDate = launchDate
    }
}

/// Error types for Chronicle collectors
public enum ChronicleCollectorError: Error, CustomStringConvertible {
    case permissionDenied(String)
    case collectorNotStarted(String)
    case collectorAlreadyStarted(String)
    case ringBufferWriteError(String)
    case systemError(String)
    case configurationError(String)
    case serializationError(String)
    
    public var description: String {
        switch self {
        case .permissionDenied(let message):
            return "Permission denied: \(message)"
        case .collectorNotStarted(let message):
            return "Collector not started: \(message)"
        case .collectorAlreadyStarted(let message):
            return "Collector already started: \(message)"
        case .ringBufferWriteError(let message):
            return "Ring buffer write error: \(message)"
        case .systemError(let message):
            return "System error: \(message)"
        case .configurationError(let message):
            return "Configuration error: \(message)"
        case .serializationError(let message):
            return "Serialization error: \(message)"
        }
    }
}

// MARK: - Extensions for CGPoint and CGRect Codable conformance

extension CGPoint: Codable {
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let x = try container.decode(CGFloat.self, forKey: .x)
        let y = try container.decode(CGFloat.self, forKey: .y)
        self.init(x: x, y: y)
    }
    
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(x, forKey: .x)
        try container.encode(y, forKey: .y)
    }
    
    private enum CodingKeys: String, CodingKey {
        case x, y
    }
}

extension CGRect: Codable {
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let x = try container.decode(CGFloat.self, forKey: .x)
        let y = try container.decode(CGFloat.self, forKey: .y)
        let width = try container.decode(CGFloat.self, forKey: .width)
        let height = try container.decode(CGFloat.self, forKey: .height)
        self.init(x: x, y: y, width: width, height: height)
    }
    
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(origin.x, forKey: .x)
        try container.encode(origin.y, forKey: .y)
        try container.encode(size.width, forKey: .width)
        try container.encode(size.height, forKey: .height)
    }
    
    private enum CodingKeys: String, CodingKey {
        case x, y, width, height
    }
}

extension CGSize: Codable {
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let width = try container.decode(CGFloat.self, forKey: .width)
        let height = try container.decode(CGFloat.self, forKey: .height)
        self.init(width: width, height: height)
    }
    
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(width, forKey: .width)
        try container.encode(height, forKey: .height)
    }
    
    private enum CodingKeys: String, CodingKey {
        case width, height
    }
}