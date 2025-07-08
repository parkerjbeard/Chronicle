import SwiftUI

struct StatusIndicator {
    enum Status {
        case good
        case warning
        case error
        case unknown
        
        var color: Color {
            switch self {
            case .good:
                return .green
            case .warning:
                return .orange
            case .error:
                return .red
            case .unknown:
                return .gray
            }
        }
        
        var systemImage: String {
            switch self {
            case .good:
                return "checkmark.circle.fill"
            case .warning:
                return "exclamationmark.triangle.fill"
            case .error:
                return "xmark.circle.fill"
            case .unknown:
                return "questionmark.circle.fill"
            }
        }
    }
    
    static func systemStatus(from status: SystemStatus) -> Status {
        let maxUsage = max(status.cpuUsage, status.memoryUsage, status.diskUsage)
        
        if maxUsage > 90 {
            return .error
        } else if maxUsage > 70 {
            return .warning
        } else {
            return .good
        }
    }
    
    static func collectorStatus(from collector: CollectorStatus) -> Status {
        switch collector.status {
        case .healthy:
            return .good
        case .warning:
            return .warning
        case .error:
            return .error
        case .disabled:
            return .unknown
        }
    }
    
    static func permissionStatus(from permissions: PermissionStatus) -> Status {
        let allPermissions = [
            permissions.accessibility,
            permissions.inputMonitoring,
            permissions.screenCapture,
            permissions.fullDiskAccess,
            permissions.microphone,
            permissions.camera
        ]
        
        if allPermissions.allSatisfy({ $0 == .granted }) {
            return .good
        } else if allPermissions.contains(.denied) {
            return .error
        } else {
            return .warning
        }
    }
    
    static func ringBufferStatus(from stats: RingBufferStats) -> Status {
        let usage = stats.usagePercentage
        
        if usage > 95 {
            return .error
        } else if usage > 80 {
            return .warning
        } else {
            return .good
        }
    }
}

extension StatusIndicator.Status {
    var description: String {
        switch self {
        case .good:
            return "Good"
        case .warning:
            return "Warning"
        case .error:
            return "Error"
        case .unknown:
            return "Unknown"
        }
    }
}