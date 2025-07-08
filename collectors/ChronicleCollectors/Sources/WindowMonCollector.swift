//
//  WindowMonCollector.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import AppKit
import CoreGraphics
import os.log

/// Window monitoring collector for tracking window focus and metadata
public class WindowMonCollector: CollectorBase {
    private var windowObserver: NSObjectProtocol?
    private var applicationObserver: NSObjectProtocol?
    private let permissionManager: PermissionManager
    private var currentWindowInfo: WindowInfo?
    private var monitoringTimer: Timer?
    private let workspace = NSWorkspace.shared
    
    public init(configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter,
                permissionManager: PermissionManager = PermissionManager()) {
        self.permissionManager = permissionManager
        
        super.init(
            identifier: "window_mon",
            displayName: "Window Monitor",
            eventTypes: [.windowFocus],
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
        
        // Set up window and application observers
        setupObservers()
        
        // Start periodic monitoring
        startPeriodicMonitoring()
        
        // Capture initial window state
        captureCurrentWindow()
        
        logger.info("Window monitor collector started successfully")
    }
    
    public override func stopCollector() throws {
        removeObservers()
        stopPeriodicMonitoring()
        
        logger.info("Window monitor collector stopped")
    }
    
    // MARK: - Observer Setup
    
    private func setupObservers() {
        // Observe application activation
        applicationObserver = workspace.notificationCenter.addObserver(
            forName: NSWorkspace.didActivateApplicationNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            self?.handleApplicationActivation(notification)
        }
        
        // Observe window changes using NSWorkspace
        windowObserver = workspace.notificationCenter.addObserver(
            forName: NSWorkspace.activeSpaceDidChangeNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            self?.handleSpaceChange(notification)
        }
    }
    
    private func removeObservers() {
        if let observer = applicationObserver {
            workspace.notificationCenter.removeObserver(observer)
            applicationObserver = nil
        }
        
        if let observer = windowObserver {
            workspace.notificationCenter.removeObserver(observer)
            windowObserver = nil
        }
    }
    
    // MARK: - Periodic Monitoring
    
    private func startPeriodicMonitoring() {
        let interval = 1.0 / currentFrameRate
        
        monitoringTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            self?.periodicWindowCheck()
        }
    }
    
    private func stopPeriodicMonitoring() {
        monitoringTimer?.invalidate()
        monitoringTimer = nil
    }
    
    private func periodicWindowCheck() {
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        captureCurrentWindow()
    }
    
    // MARK: - Event Handlers
    
    private func handleApplicationActivation(_ notification: Notification) {
        guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication else {
            return
        }
        
        logger.debug("Application activated: \(app.localizedName ?? "Unknown")")
        
        // Wait a moment for the window to become active
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            self.captureCurrentWindow()
        }
    }
    
    private func handleSpaceChange(_ notification: Notification) {
        logger.debug("Active space changed")
        
        // Wait a moment for the space change to complete
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            self.captureCurrentWindow()
        }
    }
    
    // MARK: - Window Capture
    
    private func captureCurrentWindow() {
        guard isRunning else { return }
        
        let newWindowInfo = getCurrentWindowInfo()
        
        // Check if window has actually changed
        if let current = currentWindowInfo,
           let new = newWindowInfo,
           windowsAreEqual(current, new) {
            return
        }
        
        // Create window focus event
        let eventData = WindowFocusEventData(
            windowInfo: newWindowInfo ?? createUnknownWindowInfo(),
            previousWindowInfo: currentWindowInfo,
            focusChangeReason: determineFocusChangeReason(from: currentWindowInfo, to: newWindowInfo)
        )
        
        emitWindowFocusEvent(eventData)
        
        // Update current window
        currentWindowInfo = newWindowInfo
        updateActivity()
    }
    
    private func getCurrentWindowInfo() -> WindowInfo? {
        guard let frontmostApp = workspace.frontmostApplication else {
            return nil
        }
        
        // Get window information using Accessibility API
        if let windowInfo = getAccessibilityWindowInfo(for: frontmostApp) {
            return windowInfo
        }
        
        // Fallback to CGWindowList
        return getCGWindowInfo(for: frontmostApp)
    }
    
    private func getAccessibilityWindowInfo(for app: NSRunningApplication) -> WindowInfo? {
        let appRef = AXUIElementCreateApplication(app.processIdentifier)
        
        var windowsRef: CFTypeRef?
        let result = AXUIElementCopyAttributeValue(appRef, kAXWindowsAttribute as CFString, &windowsRef)
        
        guard result == .success,
              let windows = windowsRef as? [AXUIElement] else {
            return nil
        }
        
        // Get the focused window
        var focusedWindowRef: CFTypeRef?
        let focusResult = AXUIElementCopyAttributeValue(appRef, kAXFocusedWindowAttribute as CFString, &focusedWindowRef)
        
        let focusedWindow = (focusResult == .success) ? (focusedWindowRef as? AXUIElement) : windows.first
        
        guard let window = focusedWindow else { return nil }
        
        // Get window properties
        let windowTitle = getAXStringValue(window, kAXTitleAttribute) ?? ""
        let position = getAXPointValue(window, kAXPositionAttribute) ?? CGPoint.zero
        let size = getAXSizeValue(window, kAXSizeAttribute) ?? CGSize.zero
        let isMinimized = getAXBoolValue(window, kAXMinimizedAttribute) ?? false
        
        return WindowInfo(
            windowId: 0, // Not available through Accessibility API
            processId: app.processIdentifier,
            processName: app.localizedName ?? "Unknown",
            windowTitle: windowTitle,
            bundleIdentifier: app.bundleIdentifier,
            bounds: CGRect(origin: position, size: size),
            isOnScreen: !isMinimized,
            isActive: true,
            level: 0,
            alpha: 1.0
        )
    }
    
    private func getCGWindowInfo(for app: NSRunningApplication) -> WindowInfo? {
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements, .optionOnScreenOnly)
        let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]]
        
        // Find the frontmost window for this application
        for window in windowList ?? [] {
            if let ownerPID = window[kCGWindowOwnerPID as String] as? Int32,
               ownerPID == app.processIdentifier {
                
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
                
                return WindowInfo(
                    windowId: windowId,
                    processId: app.processIdentifier,
                    processName: app.localizedName ?? "Unknown",
                    windowTitle: windowTitle,
                    bundleIdentifier: app.bundleIdentifier,
                    bounds: CGRect(x: x, y: y, width: width, height: height),
                    isOnScreen: isOnScreen,
                    isActive: true,
                    level: windowLevel,
                    alpha: alpha
                )
            }
        }
        
        return nil
    }
    
    private func createUnknownWindowInfo() -> WindowInfo {
        return WindowInfo(
            windowId: 0,
            processId: 0,
            processName: "Unknown",
            windowTitle: "Unknown",
            bundleIdentifier: nil,
            bounds: CGRect.zero,
            isOnScreen: false,
            isActive: false,
            level: 0,
            alpha: 0.0
        )
    }
    
    // MARK: - Accessibility Helpers
    
    private func getAXStringValue(_ element: AXUIElement, _ attribute: String) -> String? {
        var valueRef: CFTypeRef?
        let result = AXUIElementCopyAttributeValue(element, attribute as CFString, &valueRef)
        
        guard result == .success else { return nil }
        return valueRef as? String
    }
    
    private func getAXPointValue(_ element: AXUIElement, _ attribute: String) -> CGPoint? {
        var valueRef: CFTypeRef?
        let result = AXUIElementCopyAttributeValue(element, attribute as CFString, &valueRef)
        
        guard result == .success else { return nil }
        
        var point = CGPoint.zero
        let converted = AXValueGetValue(valueRef as! AXValue, .cgPoint, &point)
        return converted ? point : nil
    }
    
    private func getAXSizeValue(_ element: AXUIElement, _ attribute: String) -> CGSize? {
        var valueRef: CFTypeRef?
        let result = AXUIElementCopyAttributeValue(element, attribute as CFString, &valueRef)
        
        guard result == .success else { return nil }
        
        var size = CGSize.zero
        let converted = AXValueGetValue(valueRef as! AXValue, .cgSize, &size)
        return converted ? size : nil
    }
    
    private func getAXBoolValue(_ element: AXUIElement, _ attribute: String) -> Bool? {
        var valueRef: CFTypeRef?
        let result = AXUIElementCopyAttributeValue(element, attribute as CFString, &valueRef)
        
        guard result == .success else { return nil }
        return valueRef as? Bool
    }
    
    // MARK: - Utility Methods
    
    private func windowsAreEqual(_ window1: WindowInfo, _ window2: WindowInfo) -> Bool {
        return window1.windowId == window2.windowId &&
               window1.processId == window2.processId &&
               window1.windowTitle == window2.windowTitle &&
               window1.bounds.equalTo(window2.bounds)
    }
    
    private func determineFocusChangeReason(from previous: WindowInfo?, to current: WindowInfo?) -> String {
        guard let current = current else {
            return "window_closed"
        }
        
        guard let previous = previous else {
            return "initial_focus"
        }
        
        if previous.processId != current.processId {
            return "application_switch"
        } else if previous.windowId != current.windowId {
            return "window_switch"
        } else if previous.windowTitle != current.windowTitle {
            return "window_title_change"
        } else {
            return "unknown"
        }
    }
    
    private func emitWindowFocusEvent(_ eventData: WindowFocusEventData) {
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            let chronicleEvent = createEvent(type: .windowFocus, data: jsonData, metadata: [
                "process_name": eventData.windowInfo.processName,
                "window_title": eventData.windowInfo.windowTitle,
                "bundle_identifier": eventData.windowInfo.bundleIdentifier ?? "unknown",
                "focus_change_reason": eventData.focusChangeReason
            ])
            
            emitEvent(chronicleEvent)
            
        } catch {
            logger.error("Failed to encode window focus event: \(error)")
        }
    }
    
    // MARK: - Adaptive Frame Rate
    
    private var currentFrameRate: Double {
        return configuration.adaptiveFrameRate ? adaptiveFrameRate : configuration.activeFrameRate
    }
    
    private var adaptiveFrameRate: Double {
        // Window monitoring can be less frequent when idle
        let hasRecentActivity = Date().timeIntervalSince1970 - (currentWindowInfo?.processId == workspace.frontmostApplication?.processIdentifier ? 0 : configuration.idleTimeout) < configuration.idleTimeout
        
        return hasRecentActivity ? configuration.activeFrameRate : configuration.idleFrameRate
    }
}

// MARK: - Window Analysis

extension WindowMonCollector {
    /// Get detailed window information for all visible windows
    public func getAllWindowInfo() -> [WindowInfo] {
        var allWindows: [WindowInfo] = []
        
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements, .optionOnScreenOnly)
        let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]]
        
        for window in windowList ?? [] {
            if let windowInfo = parseWindowInfo(window) {
                allWindows.append(windowInfo)
            }
        }
        
        return allWindows
    }
    
    private func parseWindowInfo(_ window: [String: Any]) -> WindowInfo? {
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
            isActive: ownerPID == workspace.frontmostApplication?.processIdentifier,
            level: windowLevel,
            alpha: alpha
        )
    }
    
    /// Get window statistics
    public func getWindowStatistics() -> [String: Any] {
        let allWindows = getAllWindowInfo()
        
        var appCounts: [String: Int] = [:]
        var totalArea: Double = 0
        var visibleWindows = 0
        
        for window in allWindows {
            // Count windows per application
            let appName = window.processName
            appCounts[appName] = (appCounts[appName] ?? 0) + 1
            
            // Calculate total window area
            totalArea += Double(window.bounds.width * window.bounds.height)
            
            if window.isOnScreen && window.alpha > 0.1 {
                visibleWindows += 1
            }
        }
        
        return [
            "total_windows": allWindows.count,
            "visible_windows": visibleWindows,
            "applications_with_windows": appCounts.count,
            "total_window_area": totalArea,
            "windows_per_app": appCounts,
            "current_window": currentWindowInfo?.windowTitle ?? "None"
        ]
    }
}