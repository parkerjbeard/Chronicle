//
//  PointerMonCollector.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import CoreGraphics
import AppKit
import os.log

/// Mouse pointer monitoring collector
public class PointerMonCollector: CollectorBase {
    private var eventTap: CFMachPort?
    private let eventTypes: CGEventMask
    private let permissionManager: PermissionManager
    private var lastMouseLocation: CGPoint = CGPoint.zero
    private var lastMoveTime: Date = Date()
    private var moveBuffer: [PointerMoveEvent] = []
    private let maxBufferSize = 100
    private var isTracking = false
    
    // Movement tracking
    private struct PointerMoveEvent {
        let location: CGPoint
        let timestamp: Date
        let velocity: Double
    }
    
    public init(configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter,
                permissionManager: PermissionManager = PermissionManager()) {
        self.permissionManager = permissionManager
        
        // Set up event mask for mouse events
        self.eventTypes = CGEventMask(
            (1 << CGEventType.mouseMoved.rawValue) |
            (1 << CGEventType.leftMouseDown.rawValue) |
            (1 << CGEventType.leftMouseUp.rawValue) |
            (1 << CGEventType.rightMouseDown.rawValue) |
            (1 << CGEventType.rightMouseUp.rawValue) |
            (1 << CGEventType.otherMouseDown.rawValue) |
            (1 << CGEventType.otherMouseUp.rawValue) |
            (1 << CGEventType.leftMouseDragged.rawValue) |
            (1 << CGEventType.rightMouseDragged.rawValue) |
            (1 << CGEventType.otherMouseDragged.rawValue) |
            (1 << CGEventType.scrollWheel.rawValue)
        )
        
        super.init(
            identifier: "pointer_mon",
            displayName: "Pointer Monitor",
            eventTypes: [.pointerMove, .pointerClick],
            configuration: configuration,
            ringBufferWriter: ringBufferWriter
        )
    }
    
    // MARK: - CollectorProtocol Implementation
    
    public override func checkPermissions() -> Bool {
        return permissionManager.checkPermission(.accessibility) == .granted &&
               permissionManager.checkPermission(.inputMonitoring) == .granted
    }
    
    public override func requestPermissions() async throws {
        try await permissionManager.requestPermission(.accessibility)
        try await permissionManager.requestPermission(.inputMonitoring)
    }
    
    public override func startCollector() throws {
        guard checkPermissions() else {
            throw ChronicleCollectorError.permissionDenied("Accessibility and Input Monitoring permissions required")
        }
        
        // Create event tap
        eventTap = CGEvent.tapCreate(
            tap: .cgSessionEventTap,
            place: .headInsertEventTap,
            options: .listenOnly, // Listen only, don't modify events
            eventsOfInterest: eventTypes,
            callback: { (proxy, type, event, refcon) -> Unmanaged<CGEvent>? in
                guard let collector = Unmanaged<PointerMonCollector>.fromOpaque(refcon!).takeUnretainedValue() as PointerMonCollector? else {
                    return Unmanaged.passRetained(event)
                }
                
                collector.handlePointerEvent(proxy: proxy, type: type, event: event)
                return Unmanaged.passRetained(event)
            },
            userInfo: Unmanaged.passUnretained(self).toOpaque()
        )
        
        guard let eventTap = eventTap else {
            throw ChronicleCollectorError.systemError("Failed to create pointer event tap")
        }
        
        // Enable the event tap
        CGEvent.tapEnable(tap: eventTap, enable: true)
        
        // Add to run loop
        let runLoopSource = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, eventTap, 0)
        CFRunLoopAddSource(CFRunLoopGetCurrent(), runLoopSource, .commonModes)
        
        // Initialize tracking
        lastMouseLocation = CGPoint(x: 0, y: 0)
        isTracking = true
        
        logger.info("Pointer monitor collector started successfully")
    }
    
    public override func stopCollector() throws {
        if let eventTap = eventTap {
            CGEvent.tapEnable(tap: eventTap, enable: false)
            CFMachPortInvalidate(eventTap)
            self.eventTap = nil
        }
        
        isTracking = false
        moveBuffer.removeAll()
        
        logger.info("Pointer monitor collector stopped")
    }
    
    // MARK: - Event Handling
    
    private func handlePointerEvent(proxy: CGEventTapProxy, type: CGEventType, event: CGEvent) {
        guard isRunning && isTracking else { return }
        
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        let location = event.location
        let timestamp = Date()
        
        switch type {
        case .mouseMoved, .leftMouseDragged, .rightMouseDragged, .otherMouseDragged:
            handleMouseMove(location: location, timestamp: timestamp, isDragging: type != .mouseMoved)
            
        case .leftMouseDown, .leftMouseUp:
            handleMouseClick(location: location, button: 0, isDown: type == .leftMouseDown, event: event)
            
        case .rightMouseDown, .rightMouseUp:
            handleMouseClick(location: location, button: 1, isDown: type == .rightMouseDown, event: event)
            
        case .otherMouseDown, .otherMouseUp:
            let buttonNumber = Int(event.getIntegerValueField(.mouseEventButtonNumber))
            handleMouseClick(location: location, button: buttonNumber, isDown: type == .otherMouseDown, event: event)
            
        case .scrollWheel:
            handleScrollWheel(location: location, event: event)
            
        default:
            break
        }
    }
    
    private func handleMouseMove(location: CGPoint, timestamp: Date, isDragging: Bool) {
        let deltaX = location.x - lastMouseLocation.x
        let deltaY = location.y - lastMouseLocation.y
        let deltaTime = timestamp.timeIntervalSince(lastMoveTime)
        
        // Calculate velocity (pixels per second)
        let distance = sqrt(deltaX * deltaX + deltaY * deltaY)
        let velocity = deltaTime > 0 ? Double(distance) / deltaTime : 0.0
        
        // Get current window info
        let windowInfo = getCurrentWindowInfo(at: location)
        
        // Create move event data
        let eventData = PointerMoveEventData(
            location: location,
            previousLocation: lastMouseLocation,
            deltaX: Double(deltaX),
            deltaY: Double(deltaY),
            velocity: velocity,
            windowInfo: windowInfo
        )
        
        // Add to move buffer for analysis
        let moveEvent = PointerMoveEvent(location: location, timestamp: timestamp, velocity: velocity)
        addToMoveBuffer(moveEvent)
        
        // Only emit events at reduced frequency for moves to avoid spam
        if shouldEmitMoveEvent(velocity: velocity, deltaTime: deltaTime) {
            emitPointerMoveEvent(eventData, isDragging: isDragging)
        }
        
        // Update tracking
        lastMouseLocation = location
        lastMoveTime = timestamp
        updateActivity()
    }
    
    private func handleMouseClick(location: CGPoint, button: Int, isDown: Bool, event: CGEvent) {
        let clickCount = Int(event.getIntegerValueField(.mouseEventClickState))
        let modifierFlags = event.flags.rawValue
        
        // Get current window info
        let windowInfo = getCurrentWindowInfo(at: location)
        
        // Create click event data
        let eventData = PointerClickEventData(
            location: location,
            buttonNumber: button,
            clickCount: clickCount,
            isButtonDown: isDown,
            modifierFlags: modifierFlags,
            windowInfo: windowInfo
        )
        
        emitPointerClickEvent(eventData)
        updateActivity()
    }
    
    private func handleScrollWheel(location: CGPoint, event: CGEvent) {
        let deltaY = event.getDoubleValueField(.scrollWheelEventDeltaAxis1)
        let deltaX = event.getDoubleValueField(.scrollWheelEventDeltaAxis2)
        
        // Get current window info
        let windowInfo = getCurrentWindowInfo(at: location)
        
        // Create scroll event as a click event with special metadata
        let eventData = PointerClickEventData(
            location: location,
            buttonNumber: -1, // Special value for scroll
            clickCount: 1,
            isButtonDown: true,
            modifierFlags: event.flags.rawValue,
            windowInfo: windowInfo
        )
        
        emitPointerClickEvent(eventData, isScroll: true, scrollDeltaX: deltaX, scrollDeltaY: deltaY)
    }
    
    // MARK: - Event Emission
    
    private func emitPointerMoveEvent(_ eventData: PointerMoveEventData, isDragging: Bool = false) {
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            let chronicleEvent = createEvent(type: .pointerMove, data: jsonData, metadata: [
                "velocity": String(format: "%.2f", eventData.velocity),
                "is_dragging": String(isDragging),
                "window_title": eventData.windowInfo?.windowTitle ?? "Unknown"
            ])
            
            emitEvent(chronicleEvent)
            
        } catch {
            logger.error("Failed to encode pointer move event: \(error)")
        }
    }
    
    private func emitPointerClickEvent(_ eventData: PointerClickEventData, isScroll: Bool = false, scrollDeltaX: Double = 0, scrollDeltaY: Double = 0) {
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            var metadata: [String: String] = [
                "button_number": String(eventData.buttonNumber),
                "click_count": String(eventData.clickCount),
                "is_button_down": String(eventData.isButtonDown),
                "window_title": eventData.windowInfo?.windowTitle ?? "Unknown"
            ]
            
            if isScroll {
                metadata["event_subtype"] = "scroll"
                metadata["scroll_delta_x"] = String(format: "%.2f", scrollDeltaX)
                metadata["scroll_delta_y"] = String(format: "%.2f", scrollDeltaY)
            }
            
            let chronicleEvent = createEvent(type: .pointerClick, data: jsonData, metadata: metadata)
            
            emitEvent(chronicleEvent)
            
        } catch {
            logger.error("Failed to encode pointer click event: \(error)")
        }
    }
    
    // MARK: - Utility Methods
    
    private func getCurrentWindowInfo(at location: CGPoint) -> WindowInfo? {
        // Get window at mouse location
        let windowList = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]]
        
        for window in windowList ?? [] {
            guard let bounds = window[kCGWindowBounds as String] as? [String: Any] else { continue }
            
            let x = bounds["X"] as? CGFloat ?? 0
            let y = bounds["Y"] as? CGFloat ?? 0
            let width = bounds["Width"] as? CGFloat ?? 0
            let height = bounds["Height"] as? CGFloat ?? 0
            
            let windowRect = CGRect(x: x, y: y, width: width, height: height)
            
            if windowRect.contains(location) {
                return createWindowInfo(from: window)
            }
        }
        
        return nil
    }
    
    private func createWindowInfo(from window: [String: Any]) -> WindowInfo? {
        guard let ownerPID = window[kCGWindowOwnerPID as String] as? Int32 else {
            return nil
        }
        
        let windowId = window[kCGWindowNumber as String] as? CGWindowID ?? 0
        let windowTitle = window[kCGWindowName as String] as? String ?? ""
        let bounds = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
        
        let x = bounds["X"] as? CGFloat ?? 0
        let y = bounds["Y"] as? CGFloat ?? 0
        let width = bounds["Width"] as? CGFloat ?? 0
        let height = bounds["Height"] as? CGFloat ?? 0
        
        let windowLevel = window[kCGWindowLevel as String] as? Int ?? 0
        let alpha = window[kCGWindowAlpha as String] as? Double ?? 1.0
        let isOnScreen = window[kCGWindowIsOnscreen as String] as? Bool ?? false
        
        // Get application info
        let runningApps = NSWorkspace.shared.runningApplications
        let app = runningApps.first { $0.processIdentifier == ownerPID }
        
        return WindowInfo(
            windowId: windowId,
            processId: ownerPID,
            processName: app?.localizedName ?? "Unknown",
            windowTitle: windowTitle,
            bundleIdentifier: app?.bundleIdentifier,
            bounds: CGRect(x: x, y: y, width: width, height: height),
            isOnScreen: isOnScreen,
            isActive: ownerPID == NSWorkspace.shared.frontmostApplication?.processIdentifier,
            level: windowLevel,
            alpha: alpha
        )
    }
    
    private func shouldEmitMoveEvent(velocity: Double, deltaTime: TimeInterval) -> Bool {
        // Emit more frequently for fast movements, less for slow movements
        let baseThreshold: TimeInterval = 0.1 // 10 FPS max
        let fastThreshold: TimeInterval = 0.05 // 20 FPS for fast movements
        let velocityThreshold = 100.0 // pixels per second
        
        if velocity > velocityThreshold {
            return deltaTime >= fastThreshold
        } else {
            return deltaTime >= baseThreshold
        }
    }
    
    private func addToMoveBuffer(_ moveEvent: PointerMoveEvent) {
        moveBuffer.append(moveEvent)
        
        // Keep buffer size manageable
        if moveBuffer.count > maxBufferSize {
            moveBuffer.removeFirst()
        }
        
        // Clean old entries (older than 30 seconds)
        let cutoffTime = Date().addingTimeInterval(-30.0)
        moveBuffer.removeAll { $0.timestamp < cutoffTime }
    }
    
    // MARK: - Movement Analysis
    
    /// Get movement statistics
    public func getMovementStatistics() -> [String: Any] {
        guard !moveBuffer.isEmpty else {
            return ["error": "No movement data available"]
        }
        
        let velocities = moveBuffer.map { $0.velocity }
        let avgVelocity = velocities.reduce(0, +) / Double(velocities.count)
        let maxVelocity = velocities.max() ?? 0
        let minVelocity = velocities.min() ?? 0
        
        // Calculate total distance
        var totalDistance: Double = 0
        for i in 1..<moveBuffer.count {
            let prev = moveBuffer[i-1].location
            let curr = moveBuffer[i].location
            let dx = curr.x - prev.x
            let dy = curr.y - prev.y
            totalDistance += sqrt(Double(dx*dx + dy*dy))
        }
        
        // Calculate time span
        let timeSpan = moveBuffer.last!.timestamp.timeIntervalSince(moveBuffer.first!.timestamp)
        
        return [
            "total_moves": moveBuffer.count,
            "total_distance": totalDistance,
            "time_span": timeSpan,
            "average_velocity": avgVelocity,
            "max_velocity": maxVelocity,
            "min_velocity": minVelocity,
            "current_location": [
                "x": lastMouseLocation.x,
                "y": lastMouseLocation.y
            ]
        ]
    }
    
    /// Detect mouse patterns
    public func detectMousePatterns() -> [String] {
        var patterns: [String] = []
        
        guard moveBuffer.count >= 10 else { return patterns }
        
        // Analyze recent movements
        let recentMoves = Array(moveBuffer.suffix(10))
        
        // Check for circular motion
        if isCircularMotion(recentMoves) {
            patterns.append("circular_motion")
        }
        
        // Check for rapid back-and-forth
        if isRapidBackAndForth(recentMoves) {
            patterns.append("rapid_back_and_forth")
        }
        
        // Check for idle cursor
        if isIdleCursor(recentMoves) {
            patterns.append("idle_cursor")
        }
        
        return patterns
    }
    
    private func isCircularMotion(_ moves: [PointerMoveEvent]) -> Bool {
        // Simplified circular motion detection
        // In a real implementation, you'd use more sophisticated analysis
        return false
    }
    
    private func isRapidBackAndForth(_ moves: [PointerMoveEvent]) -> Bool {
        // Check for rapid direction changes
        var directionChanges = 0
        
        for i in 2..<moves.count {
            let prev = moves[i-2].location
            let mid = moves[i-1].location
            let curr = moves[i].location
            
            let dx1 = mid.x - prev.x
            let dx2 = curr.x - mid.x
            
            // Check for direction change in X
            if (dx1 > 0 && dx2 < 0) || (dx1 < 0 && dx2 > 0) {
                directionChanges += 1
            }
        }
        
        return directionChanges >= 3
    }
    
    private func isIdleCursor(_ moves: [PointerMoveEvent]) -> Bool {
        // Check if cursor has been mostly stationary
        let avgVelocity = moves.map { $0.velocity }.reduce(0, +) / Double(moves.count)
        return avgVelocity < 5.0 // Very low velocity threshold
    }
}

// MARK: - Gesture Recognition

extension PointerMonCollector {
    /// Simple gesture recognition
    public func recognizeGestures() -> [String] {
        var gestures: [String] = []
        
        guard moveBuffer.count >= 5 else { return gestures }
        
        let recentMoves = Array(moveBuffer.suffix(5))
        
        // Check for swipe gestures
        if let direction = detectSwipe(recentMoves) {
            gestures.append("swipe_\(direction)")
        }
        
        return gestures
    }
    
    private func detectSwipe(_ moves: [PointerMoveEvent]) -> String? {
        guard moves.count >= 3 else { return nil }
        
        let start = moves.first!.location
        let end = moves.last!.location
        
        let dx = end.x - start.x
        let dy = end.y - start.y
        
        let distance = sqrt(Double(dx*dx + dy*dy))
        
        // Minimum distance for a swipe
        guard distance > 50 else { return nil }
        
        // Determine direction
        if abs(dx) > abs(dy) {
            return dx > 0 ? "right" : "left"
        } else {
            return dy > 0 ? "down" : "up"
        }
    }
}