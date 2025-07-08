import Foundation
import UserNotifications
import SwiftUI

@MainActor
class NotificationManager: ObservableObject {
    @Published var inAppNotifications: [InAppNotification] = []
    
    private let center = UNUserNotificationCenter.current()
    
    init() {
        Task {
            await requestNotificationPermission()
        }
    }
    
    // MARK: - System Notifications
    
    func requestNotificationPermission() async {
        do {
            let granted = try await center.requestAuthorization(options: [.alert, .badge, .sound])
            if granted {
                print("Notification permission granted")
            } else {
                print("Notification permission denied")
            }
        } catch {
            print("Error requesting notification permission: \(error)")
        }
    }
    
    func showSystemNotification(title: String, body: String, category: NotificationCategory = .info) {
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = category.sound
        content.categoryIdentifier = category.rawValue
        
        let request = UNNotificationRequest(
            identifier: UUID().uuidString,
            content: content,
            trigger: nil
        )
        
        center.add(request) { error in
            if let error = error {
                print("Error showing notification: \(error)")
            }
        }
    }
    
    // MARK: - In-App Notifications
    
    func showInfo(_ message: String) {
        let notification = InAppNotification(
            message: message,
            type: .info,
            timestamp: Date()
        )
        inAppNotifications.append(notification)
        scheduleRemoval(for: notification)
    }
    
    func showWarning(_ message: String) {
        let notification = InAppNotification(
            message: message,
            type: .warning,
            timestamp: Date()
        )
        inAppNotifications.append(notification)
        scheduleRemoval(for: notification)
    }
    
    func showError(_ message: String) {
        let notification = InAppNotification(
            message: message,
            type: .error,
            timestamp: Date()
        )
        inAppNotifications.append(notification)
        scheduleRemoval(for: notification)
    }
    
    func showSuccess(_ message: String) {
        let notification = InAppNotification(
            message: message,
            type: .success,
            timestamp: Date()
        )
        inAppNotifications.append(notification)
        scheduleRemoval(for: notification)
    }
    
    func dismiss(_ notification: InAppNotification) {
        inAppNotifications.removeAll { $0.id == notification.id }
    }
    
    private func scheduleRemoval(for notification: InAppNotification) {
        Task {
            try await Task.sleep(nanoseconds: UInt64(notification.type.duration * 1_000_000_000))
            dismiss(notification)
        }
    }
    
    // MARK: - Chronicle-Specific Notifications
    
    func notifyCollectorToggled(_ collectorName: String, enabled: Bool) {
        let title = "Collector \(enabled ? "Enabled" : "Disabled")"
        let body = "\(collectorName) collector is now \(enabled ? "active" : "inactive")"
        
        showSystemNotification(title: title, body: body, category: .info)
        showInfo(body)
    }
    
    func notifyBackupCompleted(duration: TimeInterval, size: Int64) {
        let title = "Backup Completed"
        let body = "Backup finished in \(String(format: "%.1f", duration))s (\(ByteCountFormatter.string(fromByteCount: size, countStyle: .binary)))"
        
        showSystemNotification(title: title, body: body, category: .success)
        showSuccess(body)
    }
    
    func notifyPermissionRequired(_ permissionName: String) {
        let title = "Permission Required"
        let body = "\(permissionName) permission is required for full functionality"
        
        showSystemNotification(title: title, body: body, category: .warning)
        showWarning(body)
    }
    
    func notifyRingBufferFull() {
        let title = "Ring Buffer Full"
        let body = "The ring buffer is approaching capacity. Consider increasing size or enabling automatic backups."
        
        showSystemNotification(title: title, body: body, category: .warning)
        showWarning(body)
    }
    
    func notifyConnectionLost() {
        let title = "Chronicle Connection Lost"
        let body = "Unable to connect to Chronicle services. Please check if the services are running."
        
        showSystemNotification(title: title, body: body, category: .error)
        showError(body)
    }
}

// MARK: - Supporting Types

struct InAppNotification: Identifiable {
    let id = UUID()
    let message: String
    let type: NotificationType
    let timestamp: Date
}

enum NotificationType {
    case info
    case warning
    case error
    case success
    
    var color: Color {
        switch self {
        case .info:
            return .blue
        case .warning:
            return .orange
        case .error:
            return .red
        case .success:
            return .green
        }
    }
    
    var systemImage: String {
        switch self {
        case .info:
            return "info.circle.fill"
        case .warning:
            return "exclamationmark.triangle.fill"
        case .error:
            return "xmark.circle.fill"
        case .success:
            return "checkmark.circle.fill"
        }
    }
    
    var duration: TimeInterval {
        switch self {
        case .info:
            return 3.0
        case .warning:
            return 5.0
        case .error:
            return 7.0
        case .success:
            return 3.0
        }
    }
}

enum NotificationCategory: String, CaseIterable {
    case info = "INFO"
    case warning = "WARNING"
    case error = "ERROR"
    case success = "SUCCESS"
    
    var sound: UNNotificationSound {
        switch self {
        case .info:
            return .default
        case .warning:
            return .defaultCritical
        case .error:
            return .defaultCritical
        case .success:
            return .default
        }
    }
}