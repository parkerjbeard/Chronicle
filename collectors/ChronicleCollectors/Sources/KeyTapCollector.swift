//
//  KeyTapCollector.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import CoreGraphics
import AppKit
import os.log

/// Keyboard event collector using CGEventTap
public class KeyTapCollector: CollectorBase {
    private var eventTap: CFMachPort?
    private let eventTypes: CGEventMask
    private let permissionManager: PermissionManager
    private var lastKeyEvent: Date = Date()
    private var keySequenceBuffer: [CGKeyCode] = []
    private let maxSequenceLength = 10
    private let sequenceTimeout: TimeInterval = 2.0
    
    public init(configuration: CollectorConfiguration = .default,
                ringBufferWriter: PerformantRingBufferWriter,
                permissionManager: PermissionManager = PermissionManager()) {
        self.permissionManager = permissionManager
        
        // Set up event mask for keyboard events
        self.eventTypes = CGEventMask(
            (1 << CGEventType.keyDown.rawValue) |
            (1 << CGEventType.keyUp.rawValue) |
            (1 << CGEventType.flagsChanged.rawValue)
        )
        
        super.init(
            identifier: "key_tap",
            displayName: "Keyboard Events",
            eventTypes: [.keyTap],
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
            options: .defaultTap,
            eventsOfInterest: eventTypes,
            callback: { (proxy, type, event, refcon) -> Unmanaged<CGEvent>? in
                guard let collector = Unmanaged<KeyTapCollector>.fromOpaque(refcon!).takeUnretainedValue() as KeyTapCollector? else {
                    return Unmanaged.passRetained(event)
                }
                
                collector.handleKeyEvent(proxy: proxy, type: type, event: event)
                return Unmanaged.passRetained(event)
            },
            userInfo: Unmanaged.passUnretained(self).toOpaque()
        )
        
        guard let eventTap = eventTap else {
            throw ChronicleCollectorError.systemError("Failed to create event tap")
        }
        
        // Enable the event tap
        CGEvent.tapEnable(tap: eventTap, enable: true)
        
        // Add to run loop
        let runLoopSource = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, eventTap, 0)
        CFRunLoopAddSource(CFRunLoopGetCurrent(), runLoopSource, .commonModes)
        
        logger.info("Key tap collector started successfully")
    }
    
    public override func stopCollector() throws {
        if let eventTap = eventTap {
            CGEvent.tapEnable(tap: eventTap, enable: false)
            CFMachPortInvalidate(eventTap)
            self.eventTap = nil
        }
        
        logger.info("Key tap collector stopped")
    }
    
    // MARK: - Event Handling
    
    private func handleKeyEvent(proxy: CGEventTapProxy, type: CGEventType, event: CGEvent) {
        guard isRunning else { return }
        
        // Skip if sampling rate doesn't match
        if configuration.sampleRate < 1.0 && Double.random(in: 0...1) > configuration.sampleRate {
            return
        }
        
        let now = Date()
        let keyCode = event.getIntegerValueField(.keyboardEventKeycode)
        let flags = event.flags
        let location = event.location
        
        // Get current window info
        let windowInfo = getCurrentWindowInfo()
        
        // Create event data based on event type
        let eventData: KeyTapEventData
        
        switch type {
        case .keyDown:
            eventData = createKeyDownEvent(keyCode: UInt16(keyCode), flags: flags, location: location, windowInfo: windowInfo, event: event)
            
            // Update key sequence buffer
            updateKeySequenceBuffer(keyCode: CGKeyCode(keyCode), timestamp: now)
            
        case .keyUp:
            eventData = createKeyUpEvent(keyCode: UInt16(keyCode), flags: flags, location: location, windowInfo: windowInfo, event: event)
            
        case .flagsChanged:
            eventData = createFlagsChangedEvent(flags: flags, location: location, windowInfo: windowInfo)
            
        default:
            return
        }
        
        // Serialize event data
        do {
            let jsonData = try JSONEncoder().encode(eventData)
            let chronicleEvent = createEvent(type: .keyTap, data: jsonData, metadata: [
                "event_type": type.rawValue.description,
                "key_code": String(keyCode),
                "window_title": windowInfo?.windowTitle ?? "Unknown"
            ])
            
            emitEvent(chronicleEvent)
            updateActivity()
            
        } catch {
            logger.error("Failed to encode key event: \(error)")
        }
    }
    
    private func createKeyDownEvent(keyCode: UInt16, flags: CGEventFlags, location: CGPoint, windowInfo: WindowInfo?, event: CGEvent) -> KeyTapEventData {
        let characters = getCharacters(from: event)
        let charactersIgnoringModifiers = getCharactersIgnoringModifiers(from: event)
        
        return KeyTapEventData(
            keyCode: keyCode,
            modifierFlags: flags.rawValue,
            isKeyDown: true,
            characters: characters,
            charactersIgnoringModifiers: charactersIgnoringModifiers,
            location: location,
            windowInfo: windowInfo
        )
    }
    
    private func createKeyUpEvent(keyCode: UInt16, flags: CGEventFlags, location: CGPoint, windowInfo: WindowInfo?, event: CGEvent) -> KeyTapEventData {
        let characters = getCharacters(from: event)
        let charactersIgnoringModifiers = getCharactersIgnoringModifiers(from: event)
        
        return KeyTapEventData(
            keyCode: keyCode,
            modifierFlags: flags.rawValue,
            isKeyDown: false,
            characters: characters,
            charactersIgnoringModifiers: charactersIgnoringModifiers,
            location: location,
            windowInfo: windowInfo
        )
    }
    
    private func createFlagsChangedEvent(flags: CGEventFlags, location: CGPoint, windowInfo: WindowInfo?) -> KeyTapEventData {
        return KeyTapEventData(
            keyCode: 0,
            modifierFlags: flags.rawValue,
            isKeyDown: false,
            characters: nil,
            charactersIgnoringModifiers: nil,
            location: location,
            windowInfo: windowInfo
        )
    }
    
    // MARK: - Helper Methods
    
    private func getCharacters(from event: CGEvent) -> String? {
        let maxLength = 10
        var actualLength = 0
        var unicodeString = [UniChar](repeating: 0, count: maxLength)
        
        event.keyboardGetUnicodeString(maxStringLength: maxLength, actualStringLength: &actualLength, unicodeString: &unicodeString)
        
        if actualLength > 0 {
            return String(utf16CodeUnits: unicodeString, count: actualLength)
        }
        
        return nil
    }
    
    private func getCharactersIgnoringModifiers(from event: CGEvent) -> String? {
        // Create a copy of the event with modifier flags cleared
        guard let eventCopy = event.copy() else { return nil }
        
        eventCopy.flags = CGEventFlags()
        
        let maxLength = 10
        var actualLength = 0
        var unicodeString = [UniChar](repeating: 0, count: maxLength)
        
        eventCopy.keyboardGetUnicodeString(maxStringLength: maxLength, actualStringLength: &actualLength, unicodeString: &unicodeString)
        
        if actualLength > 0 {
            return String(utf16CodeUnits: unicodeString, count: actualLength)
        }
        
        return nil
    }
    
    private func getCurrentWindowInfo() -> WindowInfo? {
        guard let app = NSWorkspace.shared.frontmostApplication else { return nil }
        
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements, .optionOnScreenOnly)
        let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]]
        
        // Find the frontmost window
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
                
                return WindowInfo(
                    windowId: windowId,
                    processId: app.processIdentifier,
                    processName: app.localizedName ?? "Unknown",
                    windowTitle: windowTitle,
                    bundleIdentifier: app.bundleIdentifier,
                    bounds: CGRect(x: x, y: y, width: width, height: height),
                    isOnScreen: true,
                    isActive: true,
                    level: windowLevel,
                    alpha: alpha
                )
            }
        }
        
        return nil
    }
    
    private func updateKeySequenceBuffer(keyCode: CGKeyCode, timestamp: Date) {
        // Clean old entries
        let cutoffTime = timestamp.addingTimeInterval(-sequenceTimeout)
        keySequenceBuffer.removeAll { _ in
            // For simplicity, we'll just keep the last maxSequenceLength entries
            return false
        }
        
        // Add new key
        keySequenceBuffer.append(keyCode)
        
        // Keep buffer size manageable
        if keySequenceBuffer.count > maxSequenceLength {
            keySequenceBuffer.removeFirst()
        }
    }
    
    private func detectKeyboardShortcuts() -> [String] {
        // Detect common shortcuts based on recent key sequence
        var shortcuts: [String] = []
        
        if keySequenceBuffer.count >= 2 {
            let lastTwoKeys = Array(keySequenceBuffer.suffix(2))
            
            // Check for common shortcuts (simplified)
            if lastTwoKeys.count == 2 {
                let first = lastTwoKeys[0]
                let second = lastTwoKeys[1]
                
                // Cmd+C, Cmd+V, etc. would be detected here
                // This is a simplified implementation
                shortcuts.append("sequence_detected")
            }
        }
        
        return shortcuts
    }
}

// MARK: - Key Code Utilities

extension KeyTapCollector {
    /// Convert key code to human-readable string
    private func keyCodeToString(_ keyCode: UInt16) -> String {
        switch keyCode {
        case 0: return "A"
        case 1: return "S"
        case 2: return "D"
        case 3: return "F"
        case 4: return "H"
        case 5: return "G"
        case 6: return "Z"
        case 7: return "X"
        case 8: return "C"
        case 9: return "V"
        case 11: return "B"
        case 12: return "Q"
        case 13: return "W"
        case 14: return "E"
        case 15: return "R"
        case 16: return "Y"
        case 17: return "T"
        case 18: return "1"
        case 19: return "2"
        case 20: return "3"
        case 21: return "4"
        case 22: return "6"
        case 23: return "5"
        case 24: return "="
        case 25: return "9"
        case 26: return "7"
        case 27: return "-"
        case 28: return "8"
        case 29: return "0"
        case 30: return "]"
        case 31: return "O"
        case 32: return "U"
        case 33: return "["
        case 34: return "I"
        case 35: return "P"
        case 36: return "Return"
        case 37: return "L"
        case 38: return "J"
        case 39: return "'"
        case 40: return "K"
        case 41: return ";"
        case 42: return "\\"
        case 43: return ","
        case 44: return "/"
        case 45: return "N"
        case 46: return "M"
        case 47: return "."
        case 48: return "Tab"
        case 49: return "Space"
        case 50: return "`"
        case 51: return "Delete"
        case 53: return "Escape"
        case 54: return "Right Command"
        case 55: return "Command"
        case 56: return "Shift"
        case 57: return "Caps Lock"
        case 58: return "Option"
        case 59: return "Control"
        case 60: return "Right Shift"
        case 61: return "Right Option"
        case 62: return "Right Control"
        case 63: return "Function"
        case 64: return "F17"
        case 65: return "Keypad ."
        case 67: return "Keypad *"
        case 69: return "Keypad +"
        case 71: return "Keypad Clear"
        case 75: return "Keypad /"
        case 76: return "Keypad Enter"
        case 78: return "Keypad -"
        case 79: return "F18"
        case 80: return "F19"
        case 81: return "Keypad ="
        case 82: return "Keypad 0"
        case 83: return "Keypad 1"
        case 84: return "Keypad 2"
        case 85: return "Keypad 3"
        case 86: return "Keypad 4"
        case 87: return "Keypad 5"
        case 88: return "Keypad 6"
        case 89: return "Keypad 7"
        case 90: return "F20"
        case 91: return "Keypad 8"
        case 92: return "Keypad 9"
        case 96: return "F5"
        case 97: return "F6"
        case 98: return "F7"
        case 99: return "F3"
        case 100: return "F8"
        case 101: return "F9"
        case 103: return "F11"
        case 105: return "F13"
        case 106: return "F16"
        case 107: return "F14"
        case 109: return "F10"
        case 111: return "F12"
        case 113: return "F15"
        case 114: return "Help"
        case 115: return "Home"
        case 116: return "Page Up"
        case 117: return "Delete Forward"
        case 118: return "F4"
        case 119: return "End"
        case 120: return "F2"
        case 121: return "Page Down"
        case 122: return "F1"
        case 123: return "Left Arrow"
        case 124: return "Right Arrow"
        case 125: return "Down Arrow"
        case 126: return "Up Arrow"
        default: return "Unknown (\(keyCode))"
        }
    }
    
    /// Get modifier flags description
    private func modifierFlagsToString(_ flags: CGEventFlags) -> [String] {
        var modifiers: [String] = []
        
        if flags.contains(.maskAlphaShift) {
            modifiers.append("Caps Lock")
        }
        if flags.contains(.maskShift) {
            modifiers.append("Shift")
        }
        if flags.contains(.maskControl) {
            modifiers.append("Control")
        }
        if flags.contains(.maskAlternate) {
            modifiers.append("Option")
        }
        if flags.contains(.maskCommand) {
            modifiers.append("Command")
        }
        if flags.contains(.maskNumericPad) {
            modifiers.append("Numeric Pad")
        }
        if flags.contains(.maskHelp) {
            modifiers.append("Help")
        }
        if flags.contains(.maskSecondaryFn) {
            modifiers.append("Function")
        }
        
        return modifiers
    }
}

// MARK: - Privacy and Security

extension KeyTapCollector {
    /// Check if content should be filtered for privacy
    private func shouldFilterContent(_ content: String?) -> Bool {
        guard let content = content else { return false }
        
        // Check against privacy settings
        let privacyConfig = ConfigManager.shared.config.privacy
        
        if privacyConfig.enableSensitiveDataFiltering {
            // Check for sensitive keywords
            let lowercaseContent = content.lowercased()
            
            for keyword in privacyConfig.excludeKeywords {
                if lowercaseContent.contains(keyword.lowercased()) {
                    return true
                }
            }
            
            // Check for common sensitive patterns
            let sensitivePatterns = [
                "password",
                "passwd",
                "secret",
                "token",
                "key",
                "ssn",
                "social security",
                "credit card",
                "card number"
            ]
            
            for pattern in sensitivePatterns {
                if lowercaseContent.contains(pattern) {
                    return true
                }
            }
        }
        
        return false
    }
    
    /// Anonymize or filter sensitive content
    private func filterSensitiveContent(_ content: String?) -> String? {
        guard let content = content else { return nil }
        
        if shouldFilterContent(content) {
            return "[FILTERED]"
        }
        
        return content
    }
}