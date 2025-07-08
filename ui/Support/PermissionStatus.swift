import Foundation
import AppKit

struct PermissionStatus: Codable {
    let accessibility: PermissionState
    let inputMonitoring: PermissionState
    let screenCapture: PermissionState
    let fullDiskAccess: PermissionState
    let microphone: PermissionState
    let camera: PermissionState
    
    init() {
        self.accessibility = .unknown
        self.inputMonitoring = .unknown
        self.screenCapture = .unknown
        self.fullDiskAccess = .unknown
        self.microphone = .unknown
        self.camera = .unknown
    }
    
    enum PermissionState: String, Codable, CaseIterable {
        case granted = "granted"
        case denied = "denied"
        case notDetermined = "notDetermined"
        case unknown = "unknown"
        
        var color: Color {
            switch self {
            case .granted:
                return .green
            case .denied:
                return .red
            case .notDetermined:
                return .orange
            case .unknown:
                return .gray
            }
        }
        
        var systemImage: String {
            switch self {
            case .granted:
                return "checkmark.circle.fill"
            case .denied:
                return "xmark.circle.fill"
            case .notDetermined:
                return "clock.fill"
            case .unknown:
                return "questionmark.circle.fill"
            }
        }
        
        var description: String {
            switch self {
            case .granted:
                return "Granted"
            case .denied:
                return "Denied"
            case .notDetermined:
                return "Not Determined"
            case .unknown:
                return "Unknown"
            }
        }
    }
    
    var overallStatus: PermissionState {
        let allPermissions = [accessibility, inputMonitoring, screenCapture, fullDiskAccess, microphone, camera]
        
        if allPermissions.allSatisfy({ $0 == .granted }) {
            return .granted
        } else if allPermissions.contains(.denied) {
            return .denied
        } else if allPermissions.contains(.notDetermined) {
            return .notDetermined
        } else {
            return .unknown
        }
    }
    
    var grantedCount: Int {
        let allPermissions = [accessibility, inputMonitoring, screenCapture, fullDiskAccess, microphone, camera]
        return allPermissions.filter { $0 == .granted }.count
    }
    
    var totalCount: Int {
        return 6
    }
}

extension PermissionStatus {
    struct PermissionInfo {
        let name: String
        let state: PermissionState
        let description: String
        let systemPreferencePane: String
        let isRequired: Bool
    }
    
    var permissionInfos: [PermissionInfo] {
        return [
            PermissionInfo(
                name: "Accessibility",
                state: accessibility,
                description: "Allows monitoring of window and application activity",
                systemPreferencePane: "com.apple.preference.security",
                isRequired: true
            ),
            PermissionInfo(
                name: "Input Monitoring",
                state: inputMonitoring,
                description: "Allows monitoring of keyboard and mouse activity",
                systemPreferencePane: "com.apple.preference.security",
                isRequired: true
            ),
            PermissionInfo(
                name: "Screen Capture",
                state: screenCapture,
                description: "Allows monitoring of screen content and changes",
                systemPreferencePane: "com.apple.preference.security",
                isRequired: true
            ),
            PermissionInfo(
                name: "Full Disk Access",
                state: fullDiskAccess,
                description: "Allows complete file system monitoring",
                systemPreferencePane: "com.apple.preference.security",
                isRequired: true
            ),
            PermissionInfo(
                name: "Microphone",
                state: microphone,
                description: "Allows monitoring of audio input activity",
                systemPreferencePane: "com.apple.preference.security",
                isRequired: false
            ),
            PermissionInfo(
                name: "Camera",
                state: camera,
                description: "Allows monitoring of camera activity",
                systemPreferencePane: "com.apple.preference.security",
                isRequired: false
            )
        ]
    }
}

// MARK: - Permission Management

@MainActor
class PermissionManager: ObservableObject {
    @Published var currentStatus = PermissionStatus()
    
    func checkAllPermissions() async {
        currentStatus = await getCurrentPermissionStatus()
    }
    
    func requestPermission(_ permission: PermissionType) async -> Bool {
        switch permission {
        case .accessibility:
            return await requestAccessibilityPermission()
        case .inputMonitoring:
            return await requestInputMonitoringPermission()
        case .screenCapture:
            return await requestScreenCapturePermission()
        case .fullDiskAccess:
            return await requestFullDiskAccessPermission()
        case .microphone:
            return await requestMicrophonePermission()
        case .camera:
            return await requestCameraPermission()
        }
    }
    
    func openSystemPreferences(for permission: PermissionType) {
        let url: String
        
        switch permission {
        case .accessibility:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        case .inputMonitoring:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        case .screenCapture:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        case .fullDiskAccess:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"
        case .microphone:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        case .camera:
            url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Camera"
        }
        
        if let settingsURL = URL(string: url) {
            NSWorkspace.shared.open(settingsURL)
        }
    }
    
    // MARK: - Private Methods
    
    private func getCurrentPermissionStatus() async -> PermissionStatus {
        // In a real implementation, this would check actual system permissions
        // For now, return a mock status
        return PermissionStatus()
    }
    
    private func requestAccessibilityPermission() async -> Bool {
        // Implementation would use AXIsProcessTrusted()
        return false
    }
    
    private func requestInputMonitoringPermission() async -> Bool {
        // Implementation would use CGRequestListenEventAccess()
        return false
    }
    
    private func requestScreenCapturePermission() async -> Bool {
        // Implementation would use CGRequestScreenCaptureAccess()
        return false
    }
    
    private func requestFullDiskAccessPermission() async -> Bool {
        // Implementation would check file system access
        return false
    }
    
    private func requestMicrophonePermission() async -> Bool {
        // Implementation would use AVCaptureDevice.requestAccess()
        return false
    }
    
    private func requestCameraPermission() async -> Bool {
        // Implementation would use AVCaptureDevice.requestAccess()
        return false
    }
}

enum PermissionType: String, CaseIterable {
    case accessibility
    case inputMonitoring
    case screenCapture
    case fullDiskAccess
    case microphone
    case camera
}